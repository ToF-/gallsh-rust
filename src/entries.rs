use core::cmp::Ordering::Equal;
use crate::image::get_image_color;
use crate::entry::{original_file_path};
use core::cmp::{min};
use crate::{THUMB_SUFFIX, Entry, EntryList, make_entry, Order};
use crate::entry::{image_data_file_path, thumbnail_file_path};
use crate::rank::{Rank};
use rand::seq::SliceRandom;
use rand::{thread_rng,Rng}; 
use regex::Regex;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions, read_to_string};
use std::fs;
use std::io::{BufReader, Write};
use std::io::{Error,ErrorKind};
use std::io;
use std::path::Path;
use std::path::{PathBuf};
use thumbnailer::error::{ThumbResult, ThumbError};
use thumbnailer::{create_thumbnails, ThumbnailSize};
use walkdir::WalkDir;


const VALID_EXTENSIONS: [&'static str; 6] = ["jpg", "jpeg", "png", "JPG", "JPEG", "PNG"];
const MAX_THUMBNAILS: usize = 100;
const SELECTION_FILE_NAME: &str = "selections";

// a struct to keep track of navigating in a list of image files
#[derive(Clone, Debug)]
pub struct Entries {
    pub entry_list: EntryList,
    pub current: usize,
    pub offset: usize,
    pub maximum:  usize,
    pub start_index: Option<usize>,
    pub star3_index: Option<usize>,
    pub max_cells: usize,
    pub cells_per_row: usize,
    pub real_size: bool,
    pub register: Option<usize>,
}

fn get_or_set_image_data(file_path: &str) -> Result<(usize,Rank),String> {

    let cs_file_path = PathBuf::from(image_data_file_path(file_path));
    if cs_file_path.exists() {
        match read_to_string(cs_file_path.clone()) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok((colors,rank)) => Ok((colors,rank)),
                Err(err) => {
                    println!("error parsing {}: {}", cs_file_path.clone().to_str().unwrap(), err);
                    Err(err.to_string())
                },
            },
            Err(err) => {
                println!("error reading {}: {}", cs_file_path.to_str().unwrap(), err);
                Err(err.to_string())
            },
        }
    } else {
        match get_image_color(&original_file_path(file_path)) {
            Ok(colors) => {
                let image_data_path = image_data_file_path(file_path);
                let path = PathBuf::from(image_data_path);
                match File::create(path.clone()) {
                    Ok(output_file) => {
                        let data = (colors, Rank::NoStar);
                        match serde_json::to_writer(output_file, &data) {
                            Ok(_) => Ok((colors, Rank::NoStar)),
                            Err(err) => {
                                println!("error writing {}: {}", path.to_str().unwrap(), err);
                                Err(err.to_string())
                            },
                        }
                    },
                    Err(err) => {
                        println!("error creating file {}: {}", path.to_str().unwrap(),err);
                        Err(err.to_string())
                    },
                }
            },
            Err(err) => Err(err.to_string()),
        }
    }
}

impl Entries {
    fn new(entry_list: Vec<Entry>, grid_size: usize) -> Self {
        Entries {
            entry_list: entry_list.clone(),
            current: 0,
            offset: 0,
            maximum: entry_list.len() - 1,
            start_index: None,
            star3_index: None,
            cells_per_row: grid_size,
            max_cells: grid_size * grid_size,
            real_size: false,
            register: None,
        }
    }

    pub fn sort_by(&mut self, order: Order) {
        match order {
            Order::Colors => self.entry_list.sort_by(|a, b| { a.colors.cmp(&b.colors) }),
            Order::Date => self.entry_list.sort_by(|a, b| { a.modified_time.cmp(&b.modified_time) }),
            Order::Name => self.entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) }),
            Order::Size => self.entry_list.sort_by(|a, b| { a.file_size.cmp(&b.file_size) }),
            Order::Value => self.entry_list.sort_by(|a,b| {
                let cmp = (a.rank as usize).cmp(&(b.rank as usize));
                if cmp == Equal {
                    a.file_path.cmp(&b.file_path)
                } else {
                    cmp
                }
            }),
            Order::Random => self.entry_list.shuffle(&mut thread_rng()),
        }
    }
    
    pub fn slice(&mut self, from: Option<usize>, to: Option<usize>) {
        let start_index = match from {
            Some(n) => min(n, self.maximum),
            None => 0,
        };
        let end_index = match to {
            Some(n) => min(n, self.maximum) + 1,
            None => self.maximum + 1,
        };
        self.entry_list = self.entry_list[start_index..end_index].to_vec();
        self.maximum = self.entry_list.len() - 1;
        self.current = 0;
        self.start_index = None;
    }

    pub fn len(&self) -> usize {
        self.maximum + 1
    }

    pub fn from_directory(dir_path: &str,
        thumbnails: bool,
        opt_pattern: &Option<String>,
        opt_low_size: Option<u64>,
        opt_high_size: Option<u64>,
        from_index: Option<usize>,
        to_index: Option<usize>,
        order: Order,
        grid_size: usize) -> io::Result<Self> {
        let mut entry_list: EntryList = Vec::new();
        for dir_entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let path = dir_entry.into_path();
            let check_extension = match path.extension() {
                Some(extension) => {
                    let s = extension.to_str().unwrap();
                    VALID_EXTENSIONS.contains(&s)
                },
                None => false,
            };
            let check_pattern = path.is_file() && match opt_pattern {
                Some(pattern) => {
                    match Regex::new(pattern) {
                        Ok(re) => match re.captures(path.to_str().unwrap()) {
                            Some(_) => true,
                            None => false,
                        },
                        Err(err) => {
                            println!("error: {}",err);
                            std::process::exit(1);
                        },
                    }
                },
                None => true,
            };
            let check_thumbnails = match path.to_str().map(|filename| filename.contains(THUMB_SUFFIX)) {
                Some(result) => result == thumbnails,
                None => false,
            };
            let high_size_limit = match opt_high_size {
                Some(high) => high,
                None => std::u64::MAX,
            };
            let low_size_limit = match opt_low_size {
                Some(low) => low,
                None => 0,
            };
            if check_extension && check_pattern && check_thumbnails {
                if let Ok(metadata) = fs::metadata(&path) {
                    let file_size = metadata.len();
                    if file_size == 0 {
                        println!("file {} has a size of 0", path.to_str().unwrap())
                    };
                    let (colors,rank) = match get_or_set_image_data(path.to_str().unwrap()) {
                        Ok(data) => data,
                        Err(err) => {
                            println!("can't find image data for file {}, {}", path.to_str().unwrap(), err);
                            (0,Rank::NoStar)
                        },
                    };
                    let modified_time = metadata.modified().unwrap();
                    if low_size_limit <= file_size && file_size <= high_size_limit {
                        if let Some(full_name) = path.to_str() {
                            let entry_name = full_name.to_string().to_owned();
                            entry_list.push(make_entry(entry_name, file_size, colors, modified_time, rank));
                        }
                    }
                } else {
                    println!("can't open: {}", path.display());
                }
            }
        };
        let mut sorted_entries = Entries::new(entry_list.clone(), grid_size);
        sorted_entries.sort_by(order);
        sorted_entries.slice(from_index, to_index);
        Ok(sorted_entries)
    } 

    pub fn from_file(file_path: &str, grid_size: usize) -> io::Result<Self> {
        let mut entry_list =Vec::new();
        match fs::metadata(&file_path) {
            Ok(metadata) => {
                let file_size = metadata.len();
                let entry_name = file_path.to_string().to_owned();
                let modified_time = metadata.modified().unwrap();
                let (colors,rank) = match get_or_set_image_data(&file_path) {
                    Ok(data) => data,
                    Err(err) => {
                        println!("can't find color size of: {}, {}", file_path, err);
                        (0,Rank::NoStar)
                    },
                };
                entry_list.push(make_entry(entry_name, file_size, colors, modified_time,rank));
                Ok(Entries::new(entry_list, grid_size))
            },
            Err(err) => {
                println!("can't open: {}: {}", file_path, err);
                Err(err)
            },
        }
    }
    pub fn from_list(list_file_path: &str, order: Order, grid_size: usize) -> io::Result<Self> {
        match read_to_string(list_file_path) {
            Ok(content) => {
                let mut entry_list = Vec::new();
                let mut file_paths_set: HashSet<String> = HashSet::new();
                for path in content.lines().map(String::from).collect::<Vec<_>>() {
                    match fs::metadata(&path) {
                        Ok(metadata) => {
                            let file_size = metadata.len();
                            let entry_name = path.to_string().to_owned();
                            let modified_time = metadata.modified().unwrap();
                            let (colors,rank) = match get_or_set_image_data(path.as_str()) {
                                Ok(n) => n,
                                Err(err) => {
                                    println!("can't find color size of: {}, {}", path.as_str(), err);
                                    (0,Rank::NoStar)
                                },
                            };
                            if ! file_paths_set.contains(&entry_name) {
                                file_paths_set.insert(entry_name.clone());
                                entry_list.push(make_entry(entry_name, file_size, colors, modified_time, rank));
                            } else {
                                println!("{} already in reading list", entry_name);
                            }
                        },
                        Err(err) => {
                            println!("can't open: {}: {}", path, err);
                        }

                    }
                };
                let mut result = Self::new(entry_list.clone(), grid_size);
                result.sort_by(order);
                Ok(result)
            },
            Err(err) => {
                println!("error reading list {}: {}", list_file_path, err);
                Err(err)
            }
        }
    }

    pub fn next(&mut self) {
        self.register = None;
        self.offset = 0;
        let new_position = self.current + self.max_cells;
        self.current = if new_position <= self.maximum {
            new_position
        } else {
            0
        }
    }

    pub fn prev(&mut self) {
        self.register = None;
        self.offset = 0;
        let new_position:i32 = self.current as i32 - self.max_cells as i32;
        self.current = if new_position >= 0 {
            new_position as usize
        } else {
            self.maximum - (self.maximum % self.max_cells)
        }
    }

    pub fn random(&mut self) {
        self.register = None;
        let position = thread_rng().gen_range(0..self.maximum + 1);
        self.offset = position % self.max_cells;
        self.current = position - self.offset
    }

    pub fn jump(&mut self, position: usize) {
        if position <= self.maximum {
            self.register = None;
            self.offset = position % self.max_cells;
            self.current = position - self.offset
        } else {
            println!("index too large: {}", position);
        }
    }

    pub fn add_digit_to_register(&mut self, digit: usize) {
        self.register = if let Some(r) = self.register {
            let new = r * 10 + digit;
            if new <= self.maximum {
                Some(new)
            } else {
                Some(r)
            }
        } else {
            Some(digit)
        }
    }

    pub fn remove_digit_to_register(&mut self) {
        self.register = if let Some(n) = self.register {
            if n > 0 {
                Some(n / 10)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn go_to_register(&mut self) {
        if self.register.is_none() {
            return
        } else {
            self.jump(self.register.unwrap())
        }
    }

    pub fn status(&self) -> String {
        let position = self.current + self.offset;
        if position <= self.maximum {
            let entry_status = <Entry as Clone>::clone(&self.entry_list[position]).show_status();
            format!("{}/{}  {} {} {}",
                position,
                self.maximum,
                entry_status,
                if self.register.is_none() { String::from("") } else { format!("{}", self.register.unwrap()) },
                if self.real_size { "*" } else { "" })
        } else {
            println!("unexpected: position + offset = {} > maximum {}", position, self.maximum);
            "".to_string()
        }
    }

    pub fn toggle_select_area(&mut self) {
        let position = self.current + self.offset;
        if position <= self.maximum {
            if self.entry_list[position].to_select {
                return
            } else {
                if self.start_index.is_none() {
                    self.start_index = Some(position)
                } else {
                    let mut start = self.start_index.unwrap();
                    let mut end = position;
                    if start > end {
                        let x = start;
                        start = end;
                        end = x;
                    };
                    for i in start..end+1 {
                        self.entry_list[i].to_select = true
                    };
                    self.start_index = None
                }
            }
        }
    }
    pub fn toggle_rank_area(&mut self, rank: Rank) {
        let position = self.current + self.offset;
        if position <= self.maximum {
            if self.entry_list[position].rank == Rank::ThreeStars {
                return
            } else {
                if self.star3_index.is_none() {
                    self.star3_index = Some(position)
                } else {
                    let mut start = self.star3_index.unwrap();
                    let mut end = position;
                    if start > end {
                        let x = start;
                        start = end;
                        end = x;
                    };
                    for i in start..end+1 {
                        self.entry_list[i].rank = rank
                    };
                    self.star3_index = None
                }
            }
        }
    }

    pub fn entry(&self) -> Entry {
        let position = self.current + self.offset;
        if position <= self.maximum {
            self.entry_list[position].clone()
        } else {
            self.entry_list[self.current].clone()
        }
    }

    pub fn set_grid_select(&mut self) {
        for i in 0..MAX_THUMBNAILS {
            let position = self.current + i;
            if position <= self.maximum {
                self.entry_list[position].to_select = true;
            }
        }
    }

    pub fn unset_grid_ranks(&mut self) {
        for i in 0..MAX_THUMBNAILS {
            let position = self.current + i;
            if position <= self.maximum {
                self.entry_list[position].rank = Rank::NoStar;
            }
        }
    }

    pub fn reset_all_select(&mut self) {
        for i in 0..self.maximum+1 {
            self.entry_list[i].to_select = false;
        };
        self.start_index = None;
    }

    pub fn reset_grid_select(&mut self) {
        for i in 0..MAX_THUMBNAILS {
            let position = self.current + i;
            if position <= self.maximum {
                self.entry_list[position].to_select = false;
            }
        };
        self.start_index = None;
    }

    pub fn toggle_real_size(&mut self) {
        self.real_size = !self.real_size;
    }

    pub fn toggle_select(&mut self) {
        let position = self.current + self.offset;
        if position <= self.maximum {
        self.entry_list[position].to_select = ! self.entry_list[position].to_select
        }
    }

    pub fn set_rank(&mut self, rank: Rank) {
        let position = self.current + self.offset;
        if position <= self.maximum {
            self.entry_list[position].rank = rank;
        }
    }
    pub fn save_marked_file_list(&self, selection: &Vec<&Entry>, dest_file_path: &str, thumbnails: bool) {
        let result = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dest_file_path);
        if let Ok(mut file) = result {
            for e in selection.iter() {
                let entry = *e;
                let file_path = if thumbnails {
                     entry.original_file_path()
                } else { entry.file_path.to_string() };
                println!("saving {} for reference", file_path);
                let _ = file.write(format!("{}\n", file_path).as_bytes());
            }
        }
    }

    pub fn save_marked_file_lists(&self, thumbnails: bool) {
        self.save_marked_file_list(&self.entry_list.iter().filter(|e| e.to_select).collect(), SELECTION_FILE_NAME, thumbnails)
    }

    pub fn save_updated_rank_entries(&self, selection: Vec<&Entry>) {
        for e in selection.iter() {
            let entry = *e;
            let image_data_path = image_data_file_path(&entry.file_path);
            let path = PathBuf::from(image_data_path);
            match File::create(&path) {
                Ok(output_file) => {
                    let data = (entry.colors,entry.rank);
                    match serde_json::to_writer(output_file, &data) {
                        Ok(_) => { },
                        Err(err) => {
                            println!("error writing {}: {}", path.to_str().unwrap(), err);
                        },
                    }
                },
                Err(err) => {
                    println!("error creating file {}: {}", path.to_str().unwrap(),err);
                },
            }
        }
    }

    pub fn save_updated_ranks(&self) {
        self.save_updated_rank_entries(self.entry_list.iter().filter(|e| e.rank != e.initial_rank).collect())
    }

    pub fn set_selected_images(&mut self) {
        match read_to_string(SELECTION_FILE_NAME) {
            Ok(content) => {
                for path in content.lines().map(String::from).collect::<Vec<_>>() {
                    let mut iter = self.entry_list.iter_mut();
                    match iter.find(|e| e.file_path.to_string() == path || e.file_path.to_string() == thumbnail_file_path(&path)) {
                        Some(entry) => { 
                            println!("selected: {}", entry.file_path);
                            entry.to_select = true
                        },
                        _ => { },
                    }
                }
            },
            Err(_) => { },
        }
    }

    pub fn copy_file_to_target_directory(file_path: &Path, target_directory: &Path) -> Result<u64,Error> {
        let file_name = file_path.file_name().unwrap();
        let target_file_path = target_directory.join(file_name);
        println!("copy {} to {}", file_path.display(), target_file_path.display());
        std::fs::copy(file_path, target_file_path)
    }

    pub fn copy_selection(&mut self, target: &str) {
        let target_path = Path::new(target);
        if target_path.exists() {
            let selection: Vec<&Entry> = self.entry_list.iter().filter(|e| e.to_select).collect();
            for e in selection.iter() {
                let entry = *e;
                let file_name = original_file_path(&entry.file_path);
                let thumbnail_name = thumbnail_file_path(&file_name);
                let image_data_name = image_data_file_path(&file_name);
                let file_path = Path::new(&file_name);
                let thumbnail_path = Path::new(&thumbnail_name);
                let image_data_path = Path::new(&image_data_name);
                println!("about to copy seletion: {}", file_path.display());
                match Entries::copy_file_to_target_directory(file_path, target_path) {
                    Ok(_) => match Entries::copy_file_to_target_directory(thumbnail_path, target_path) {
                        Ok(_) => match Entries::copy_file_to_target_directory(image_data_path, target_path) {
                            Ok(_) => {},
                            Err(err) => println!("error: {}", err),
                        },
                        Err(err) => println!("error: {}", err),
                    },
                    Err(err) => println!("error: {}", err),
                }

            }
        } else {
            println!("directory doesn't exist: {}", target)
        }
    }

}


pub fn create_thumbnail(source: String, target: String, number: usize, total: usize) -> ThumbResult<()> {
    println!("creating thumbnails {:6}/{:6} {}", number, total, &target);
    match File::open(&source) {
        Err(err) => {
            println!("error opening file {}: {}", source, err);
            return Err(ThumbError::IO(err))
        },
        Ok(input_file) => {
            let source_path = Path::new(source.as_str());
            let source_extension = match source_path.extension().and_then(OsStr::to_str) {
                None => {
                    println!("error: file {} has no extension", &source);
                    return Err(ThumbError::IO(Error::new(ErrorKind::Other, "no extension")))
                },
                Some(s) => s,
            };
            let reader = BufReader::new(input_file);
            let output_file = match File::create(&target) {
                Err(err) => {
                    println!("error while creating file {}: {}", &target, err);
                    return Err(ThumbError::IO(err))
                },
                Ok(file) => file,
            };
            write_thumbnail(reader, source_extension, output_file)
        },
    }
}


pub fn write_thumbnail<R: std::io::Seek + std::io::Read>(reader: BufReader<R>, extension: &str, mut output_file: File) -> ThumbResult<()> {
    let mime = match extension {
        "jpg" | "jpeg" | "JPG" | "JPEG" => mime::IMAGE_JPEG,
        "png" | "PNG" => mime::IMAGE_PNG,
        _ => panic!("wrong extension"),
    };
    let mut thumbnails = match create_thumbnails(reader, mime, [ThumbnailSize::Small]) {
        Ok(tns) => tns,
        Err(err) => {
            println!("error while creating thumbnails:{:?}", err);
            return Err(err)
        },
    };
    let thumbnail = thumbnails.pop().unwrap();
    let write_result = match extension {
        "jpg" | "jpeg" | "JPG" | "JPEG" => thumbnail.write_jpeg(&mut output_file,255),
        "png" | "PNG" => thumbnail.write_png(&mut output_file),
        _ => panic!("wrong extension"),
    };
    match write_result {
        Err(err) => {
            println!("error while writing ihunbnail:{}", err);
            Err(err)
        },
        ok => ok,
    }
}

pub fn update_thumbnails(dir_path: &str) -> ThumbResult<(usize,usize)> {
    let mut image_entry_list = match Entries::from_directory(dir_path, false, &None, None, None, None, None, Order::Name, 1) {
        Ok(image_entries) => image_entries.entry_list.clone(),
        Err(err) => return Err(ThumbError::IO(err)),
    };
    let mut thumbnail_entry_list = match  Entries::from_directory(dir_path, true, &None, None, None, None, None, Order::Name, 1) {
        Ok(thumbnail_entries) => thumbnail_entries.entry_list.clone(),
        Err(err) => return Err(ThumbError::IO(err)),
    };
    image_entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) });
    thumbnail_entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) });
    let mut number: usize = 0;
    let mut created: usize = 0;
    let total_images = image_entry_list.len();
    for entry in image_entry_list {
        let source = &entry.file_path;
        let target = entry.thumbnail_file_path();
        if let Err(_) = thumbnail_entry_list.binary_search_by(|probe| probe.file_path.to_string().cmp(&target)) {
            let _ = create_thumbnail(source.to_string(), target, number, total_images);
            created += 1;
        } else { }
        number += 1;
    };
    let mut deleted: usize = 0;
    for entry in thumbnail_entry_list {
        let source = entry.file_path.to_string();
        let target = entry.original_file_path();
        let image_path = PathBuf::from(&target);
        if ! image_path.exists() {
            println!("deleting thumbnails {} with no matching image", source.clone());
            match std::fs::remove_file(&source) {
                Err(err) => {
                    println!("error while deleting file {}: {}", source, err);
                },
                Ok(_) => {},
            };
            deleted += 1;
        }
        let data_path = PathBuf::from(image_data_file_path(&target));
        if ! image_path.exists() {
            println!("deleting data file {} with no matching image", data_path.display());
            match std::fs::remove_file(&data_path) {
                Err(err) => {
                    println!("error while deleting file {}: {}", data_path.display(), err);
                },
                Ok(_) => {},
            };
        }
    };
    Ok((created,deleted))
}
