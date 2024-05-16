use std::fs::OpenOptions;
use rand::{thread_rng,Rng}; 
use std::io::Write;
use std::time::SystemTime;
use std::path::{PathBuf};
use std::process::exit;
use clap::{Parser,ValueEnum};
use clap_num::number_range;
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::EventControllerMotion;
use gtk::prelude::*;
use gtk::traits::WidgetExt;
use gtk::{self, Application, ScrolledWindow, gdk, glib, Grid, Picture};
use mime;
use std::cell::{Ref,OnceCell,RefCell, RefMut};
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::fs::read_to_string;
use std::fs;
use std::io::BufReader;
use std::io::{Error,ErrorKind};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration};
use thumbnailer::error::{ThumbResult, ThumbError};
use thumbnailer::{create_thumbnails, ThumbnailSize};
use walkdir::WalkDir;
const FIRST_CELL: usize = 0;
const MAX_THUMBNAILS :usize = 100;

fn append_thumb_suffix(file_name: &str) -> String {
    let file_path = PathBuf::from(file_name);
    let parent = file_path.parent().unwrap();
    let extension = file_path.extension().unwrap();
    let file_stem = file_path.file_stem().unwrap();
    let new_file_name = format!("{}THUMB.{}", file_stem.to_str().unwrap(), extension.to_str().unwrap());
    let new_path = parent.join(new_file_name);
    new_path.to_str().unwrap().to_string()
}

fn remove_thumb_suffix(file_name: &str) -> String {
    let file_path = PathBuf::from(file_name);
    let parent = file_path.parent().unwrap();
    let extension = file_path.extension().unwrap();
    let file_stem = file_path.file_stem().unwrap().to_str().unwrap();
    let new_file_stem = match file_stem.strip_suffix("THUMB") {
        Some(s) => s,
        None => &file_stem,
    };
    let new_file_name = format!("{}.{}", new_file_stem, extension.to_str().unwrap());
    let new_path = parent.join(new_file_name);
    new_path.to_str().unwrap().to_string()
}

#[derive(Clone, Debug)]
struct Entry {
    file_path: String,
    file_size: u64,
    modified_time: SystemTime,
    to_select: bool,
}

type EntryList = Vec<Entry>;

fn make_entry(s:String, l:u64, t:SystemTime) -> Entry {
    return Entry { 
        file_path: s.clone(),
        file_size: l,
        modified_time: t,
        to_select: false,
    }
}

impl Entry {
    fn show_status(self,) -> String {
        format!("{} {} [{}]",
            self.file_path,
            if self.to_select { "△" } else { "" },
            self.file_size)
    }
}

// a struct to keep track of navigating in a list of image files
#[derive(Clone, Debug)]
struct Entries {
    entry_list: Vec<Entry>,
    current: usize,
    maximum:  usize,
    start_index: Option<usize>,
    end_index: Option<usize>,
    grid_size: usize,
    max_cells: usize,
    real_size: bool,
    register: Option<usize>,
}

impl Entries {
    fn new(entry_list: Vec<Entry>, grid_size: usize) -> Self {
        Entries {
            entry_list: entry_list.clone(),
            current: 0,
            maximum: entry_list.len() - 1,
            start_index: None,
            end_index: None,
            grid_size: grid_size,
            max_cells: grid_size * grid_size,
            real_size: false,
            register: None,
        }
    }

    fn next(&mut self) {
        self.register = None;
        if (self.maximum + 1) > self.max_cells {
            self.current = (self.current + self.max_cells) % (self.maximum + 1)
        }
    }

    fn prev(&mut self) {
        self.register = None;
        if self.maximum <= self.max_cells { return };
        if self.max_cells >= self.maximum {
            return
        };
        let mut next_pos = self.current - self.max_cells;
        if next_pos > self.maximum {
            next_pos = self.maximum - (usize::MAX - next_pos)
        }
        self.current = next_pos;
        self.register = None;
    }

    fn random(&mut self) {
        self.register = None;
        let position = thread_rng().gen_range(0..self.maximum + 1);
        self.current = position;
    }

    fn jump(&mut self, value: usize) {
        if value <= self.maximum {
            self.current = value
        } else {
            println!("index too large: {}", value);
        }
    }

    fn add_digit_to_resiter(&mut self, digit: usize) {
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

    fn remove_digit_to_register(&mut self) {
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

    fn go_to_register(&mut self) {
        if !self.register.is_none() {
            self.current = self.register.unwrap()
        };
        self.register = None;
    }

    fn show_status(self, offset: usize) -> String {
        format!("{} {} {} {}",
            self.current + offset,
            self.clone().offset_entry(offset).show_status(),
            if self.register.is_none() { String::from("") } else { format!("{}", self.register.unwrap()) },
            if self.real_size { "*" } else { "" })
    }

    fn offset_entry(self, offset: usize) -> Entry {
        let start = self.current;
        let position = (start + offset) % (self.maximum + 1);
        self.entry_list[position].clone()
    }

    fn start_area(&mut self) {
        if let Some(end_index) = self.end_index {
            if self.current <= end_index {
                self.set_to_select(self.current, end_index);
            }
        } else {
            self.start_index = Some(self.current)
        } 
    }

    fn start_area_with_offset(&mut self, offset: usize) {
        if let Some(end_index) = self.end_index {
            if self.current + offset <= end_index {
                self.set_to_select(self.current + offset, end_index)
            }
        } else {
        self.start_index = Some(self.current + offset)
        }
    }

    fn end_area(&mut self) {
        if let Some(start_index) = self.start_index {
            if self.current >= start_index {
                self.set_to_select(start_index, self.current);
                self.end_index = Some(self.current);
            }
        } else {
            self.end_index = Some(self.current)
        }
    }
    fn end_area_with_offset(&mut self, offset: usize) {
        if let Some(start_index) = self.start_index {
            if self.current + offset >= start_index {
                self.end_index = Some(self.current + offset);
                self.set_to_select(start_index, self.current + offset)
            }
        } else {
            self.end_index = Some(self.current + offset)
        }
    }

    fn set_to_select(&mut self, start: usize, end: usize) {
        if self.start_index.is_none() || self.end_index.is_none() { return };
        for i in start..end+1 {
            self.entry_list[i].to_select = true;
        }
    }

    fn reset_all_select(&mut self) {
        for i in 0..self.maximum+1 {
            self.entry_list[i].to_select = false;
        };
        self.start_index = None;
        self.end_index = None;
    }

    fn reset_grid_select(&mut self) {
        for i in 0..MAX_THUMBNAILS {
            self.entry_list[self.current+i].to_select = false;
        };
        self.start_index = None;
        self.end_index = None;
    }

    fn selection_size(self) -> usize {
        let mut result = 0;
        for i in 0..self.maximum+1 {
            if self.entry_list[i].to_select {
                result += 1
            }
        };
        result
    }
    
    fn toggle_real_size(&mut self) {
        self.real_size = !self.real_size;
    }

    fn toggle_to_select_with_offset(&mut self, offset: usize) {
        let position = (self.current + offset) % (self.maximum + 1);
        self.entry_list[position].to_select = ! self.entry_list[position].to_select
    }

    fn save_marked_file_list(&mut self, selection: Vec<&Entry>, dest_file_path: &str, thumbnails: bool) {
        if selection.len() > 0 {
            let result = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(dest_file_path);
            if let Ok(mut file) = result {
                for entry in selection.iter() {
                    let file_path = if thumbnails {
                        remove_thumb_suffix(&entry.file_path.clone())
                    } else { entry.file_path.clone() };
                    println!("saving {} for reference", file_path);
                    let _ = file.write(format!("{}\n", file_path).as_bytes());
                }
            }
        }
    }

    fn save_marked_file_lists(&mut self, thumbnails: bool) {
        let entry_list = &self.entry_list.clone();
        let _ = &self.save_marked_file_list(entry_list.iter().filter(|e| e.to_select).collect(), "selections", thumbnails);
    }
}

fn get_files_from_reading_list(reading_list: &String) -> io::Result<EntryList> {
    match read_to_string(reading_list) {
        Ok(content) => {
            let mut entry_list: EntryList = Vec::new();
            let mut filenames: HashSet<String> = HashSet::new();
            for path in content.lines().map(String::from).collect::<Vec<_>>() {
                match fs::metadata(&path) {
                    Ok(metadata) => {
                        let file_size = metadata.len();
                        let entry_name = path.to_string().to_owned();
                        let modified_time = metadata.modified().unwrap();
                        if ! filenames.contains(&entry_name) {
                            filenames.insert(entry_name.clone());
                            entry_list.push(make_entry(entry_name, file_size, modified_time));
                        } else {
                            println!("{} already in reading list", entry_name);
                        }
                    }
                    Err(err) => {
                        println!("can't open: {}: {}", path, err);
                    }
                }
            };
            Ok(entry_list)
        },
        Err(msg) => Err(msg)
    }

}

fn create_thumbnail(source: String, target: String, number: usize, total: usize) -> ThumbResult<()> {
    println!("creating thumbnails {:6}/{:6} {}", number, total, target.clone());
    match File::open(source.clone()) {
        Err(err) => {
            println!("error opening file {}: {}", source, err);
            return Err(ThumbError::IO(err))
        },
        Ok(input_file) => {
            let source_path = Path::new(source.as_str());
            let source_extension = match source_path.extension().and_then(OsStr::to_str) {
                None => {
                    println!("error: file {} has no extension", source.clone());
                    return Err(ThumbError::IO(Error::new(ErrorKind::Other, "no extension")))
                },
                Some(s) => s,
            };
            let reader = BufReader::new(input_file);
            let output_file = match File::create(target.clone()) {
                Err(err) => {
                    println!("error while creating file {}: {}",
                        target.clone(),
                        err);
                    return Err(ThumbError::IO(err))
                },
                Ok(file) => file,
            };
            write_thumbnail(reader, source_extension, output_file)
        },
    }
}
fn write_thumbnail<R: std::io::Seek + std::io::Read>(reader: BufReader<R>, extension: &str, mut output_file: File) -> ThumbResult<()> {
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
fn update_thumbnails(dir_path: &str) -> ThumbResult<(usize,usize)> {
    let result_images = get_files_in_directory(dir_path, false, &None, None, None);
    let mut image_entry_list = match result_images {
        Ok(entry_list) => entry_list,
        Err(err) => return Err(ThumbError::IO(err)),
    };
    let result_thumbnails = get_files_in_directory(dir_path, true, &None, None, None);
    let mut thumbnail_entry_list = match result_thumbnails {
        Ok(entry_list) => entry_list,
        Err(err) => return Err(ThumbError::IO(err)),
    };
    image_entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) });
    thumbnail_entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) });
    let mut number: usize = 0;
    let mut created: usize = 0;
    let total_images = image_entry_list.len();
    for entry in image_entry_list {
        let source = entry.file_path.clone();
        let target = append_thumb_suffix(&source);
        if let Err(_) = thumbnail_entry_list.binary_search_by(|probe|
            probe.file_path.cmp(&target)) {
            let _ = create_thumbnail(source, target, number, total_images);
            created += 1;
        } else { }
        number += 1;
    };
    let mut deleted: usize = 0;
    for entry in thumbnail_entry_list {
        let source = entry.file_path.clone();
        let target = remove_thumb_suffix(&source);
        let image_path = PathBuf::from(target.clone());
        if ! image_path.exists() {
            println!("deleting thumbnails {} with no matching image", source.clone());
            match std::fs::remove_file(source.clone()) {
                Err(err) => {
                    println!("error while deleting file {}: {}", source, err);
                },
                Ok(_) => {},
            };
            deleted += 1;
        }
    }
    Ok((created,deleted))
}

fn get_file(file_path: &str) -> io::Result<EntryList> {
    let mut entry_list: EntryList = Vec::new();
    if let Ok(metadata) = fs::metadata(&file_path) {
        let file_size = metadata.len();
        let modified_time = metadata.modified().unwrap();
        let entry_name = file_path.to_string().to_owned();
        entry_list.push(make_entry(entry_name, file_size, modified_time));
    } else {
        println!("can't open: {}", file_path);
    };
    Ok(entry_list)
}
fn get_files_in_directory(dir_path: &str, thumbnails_only: bool, opt_pattern: &Option<String>, opt_low_size: Option<u64>, opt_high_size: Option<u64>) -> io::Result<EntryList> {
    let mut entry_list: EntryList = Vec::new();
    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.into_path();
        let valid_ext = if let Some(ext) = path.extension() {
            ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "JPG" || ext == "JPEG" || ext == "PNG"
        } else {
            false
        };
        let pattern_present = if let Some(pattern) = opt_pattern {
            path.is_file() && path.to_str().map(|filename| filename.contains(pattern)) == Some(true)
        } else {
            path.is_file()
        };
        let check_thumbnails = pattern_present && path.to_str().map(|filename| filename.contains("THUMB")) == Some(thumbnails_only);
        let low_size_limit = if let Some(low) = opt_low_size {
            low
        } else {
            0
        };
        let high_size_limit = if let Some(high) = opt_high_size {
            high
        } else {
            std::u64::MAX
        };
        if valid_ext && pattern_present && check_thumbnails {
            if let Ok(metadata) = fs::metadata(&path) {
                let file_size = metadata.len();
                if file_size == 0 {
                    println!("file {} has a size of 0", path.to_str().unwrap())
                };
                let modified_time = metadata.modified().unwrap();
                if low_size_limit <= file_size && file_size <= high_size_limit  {
                    if let Some(full_name) = path.to_str() {
                        let entry_name = full_name.to_string().to_owned();
                        entry_list.push(make_entry(entry_name, file_size, modified_time));
                    }
                }
            } else {
                println!("can't open: {}", path.display());
            }
        }
    };
    Ok(entry_list)
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Order {
    Name, Size, Date,
}

#[derive(Debug, PartialEq, Eq)]
struct ParseOrderError;

fn less_than_11(s: &str) -> Result<usize, String> {
    number_range(s,1,10)
}

// declarative setting of arguments
/// Gallery Show
#[derive(Parser, Clone, Debug)]
#[command(infer_subcommands = true, infer_long_args = true, author, version, about, long_about = None)]
/// Pattern that displayed files must have
struct Args {
    #[arg(short, long)]
    pattern: Option<String>,

    /// Maximized window
    #[arg(short, long, default_value_t = false)]
    maximized: bool,

    /// Ordered display (or random)
    #[arg(short, long,value_name("order"),value_parser(clap::value_parser!(Order)))]
    ordered: Option<Order>,

    /// Timer delay for next picture
    #[arg(long)]
    timer: Option<u64>,

    /// Directory to search
    #[arg(short, long)]
    directory: Option<String>,

    /// Selection File
    #[arg(short, long)]
    selection: Option<String>,

    /// Reading List
    #[arg(short, long)]
    reading: Option<String>,

    /// Index of first image to read 
    #[arg(short, long)]
    index: Option<usize>,

    /// Grid Size
    #[arg(short, long, value_parser=less_than_11)]
    grid: Option<usize>,

    /// Low Limit on file size
    #[arg(short, long)]
    low: Option<u64>,

    /// High Limit on file size
    #[arg(short, long)]
    high: Option<u64>,

    /// File to view
    #[arg(short, long)]
    file: Option<String>, 

    /// Thumbnails only
    #[arg(long)]
    thumbnails: bool,

    /// Update thumbnails and then quit
    #[arg(long)]
    update_thumbnails: bool,

    /// Window width (and height)
    #[arg(short, long, default_value_t = 1000)]
    width: i32,
}

const DEFAULT_DIR :&str  = "images/";
const ENV_VARIABLE :&str = "GALLSHDIR";

fn main() {

    let args = Args::parse();
    let gallshdir = env::var(ENV_VARIABLE);

    // build an application with some css characteristics
    let application = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    application.connect_startup(|_| {
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data("window { background-color:black;} image { margin:1em ; }");
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css_provider,
            1000,
        );
    });

    // clone! passes a strong reference to pattern in the closure that activates the application
    application.connect_activate(clone!(@strong args => move |application: &gtk::Application| { 

        let order: Order = if let Some(result) = args.clone().ordered { result } else { Order::Name };
        let pattern = &args.pattern;
        let path = if let Some(directory_arg) = &args.directory {
            String::from(directory_arg)
        } else if let Ok(standard_dir) = &gallshdir {
            String::from(standard_dir)
        } else {
            println!("GALLSHDIR variable not set. Using {} as default.", DEFAULT_DIR);
            String::from(DEFAULT_DIR)
        };

        let reading_list = &args.reading;

        let grid_size = if args.thumbnails && args.grid == None {
            10
        } else {
            if let Some(size) = args.grid { 
                if size <= 10 {
                    size
                } else {
                    if args.thumbnails { 10 } else { 1 }
                }
            } else { 1 }
        };

        if let Some(reading_list_file) = reading_list {
            println!("searching images from the {} reading list", reading_list_file)
        } else {
            println!("searching images in {}", path)
        };


        if args.update_thumbnails {
            println!("updating thumbnails...");
            if let Ok((created, deleted)) = update_thumbnails(&path) {
                println!("{created} thumbnails added, {deleted} thumbnails deleted");
            }
            std::process::exit(0);
        }
        // get all the entries in the directory that match pattern (or all if None) or from a
        // reading list
        let mut entry_list = if let Some(reading_list_filename) = &reading_list {
            match get_files_from_reading_list(reading_list_filename) {
                Err(msg) => panic!("{}", msg),
                Ok(result) => result,
            }
        } else {
            if let Some(file) = &args.file {
                match get_file(file) {
                    Err(msg) => panic!("{}", msg),
                    Ok(result) => result,
                } 
            } else {
                match get_files_in_directory(&path, args.thumbnails, &pattern, args.low, args.high) {
                    Err(msg) => panic!("{}", msg),
                    Ok(result) => result,
                }
            }
        };
        match order {
            Order::Size => entry_list.sort_by(|a, b| { a.file_size.cmp(&b.file_size) }),
            Order::Date => entry_list.sort_by(|a, b| { a.modified_time.cmp(&b.modified_time) }),
            Order::Name => entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) }),
        }

        println!("{} files selected", entry_list.len());
        if entry_list.len() == 0 {
            application.quit();
            return
        }

        let mut entries = Entries::new(entry_list.clone(), grid_size);
        if let None = args.ordered {
            entries.random()
        };
        if let Some(index_number) = args.index {
            entries.jump(index_number);
        }
        let entries_rc = Rc::new(RefCell::new(entries));
        let entry_list_rc = Rc::new(OnceCell::<EntryList>::new());
        let offset_rc = Rc::new(RefCell::new(0));
        entry_list_rc.get_or_init(|| entry_list.clone());

        let width = if args.width < 3000 && args.width > 100 {
            args.width
        } else { 1000 } ;
        let height = width;
        // build the main window
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .default_width(width)
            .default_height(height)
            .build();

        let grid_scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .name("grid")
            .build();

        let view_scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .name("view")
            .build();

        let view = Grid::new();
        view.set_row_homogeneous(true);
        view.set_column_homogeneous(true);
        view.set_hexpand(true);
        view.set_vexpand(true);
        let stack = gtk::Stack::new();
        let image_view = Picture::new();
        let view_gesture = gtk::GestureClick::new();
        view_gesture.set_button(1);
        view_gesture.connect_pressed(clone!(@strong entries_rc, @strong stack, @strong grid_scrolled_window, @strong window => move |_,_, _, _| {
            stack.set_visible_child(&grid_scrolled_window);
        }));
        image_view.add_controller(view_gesture);
        view.attach(&image_view, 0, 0, 1, 1);
        view_scrolled_window.set_child(Some(&view));

        let grid = Grid::new();
        let _ = stack.add_child(&grid_scrolled_window);
        let _ = stack.add_child(&view_scrolled_window);
        window.set_child(Some(&stack));
        stack.set_visible_child(&view_scrolled_window);
        stack.set_visible_child(&grid_scrolled_window);
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(true);
        grid.set_hexpand(true);
        grid.set_vexpand(true);
        for row in 0 .. grid_size {
            for col in 0 .. grid_size {
                let image = Picture::new();
                grid.attach(&image, row as i32, col as i32, 1, 1);

                let select_gesture = gtk::GestureClick::new();
                select_gesture.set_button(3);
                select_gesture.connect_pressed(clone!(@strong entry_list_rc, @strong entries_rc, @strong grid, @strong window => move |_,_, _, _| {
                    let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                    let offset = col * grid_size + row;
                    if entries.clone().offset_entry(offset).to_select {
                        entries.toggle_to_select_with_offset(offset)
                    } else {
                        if ! (entries.start_index.is_none() || entries.end_index.is_none()) {
                            entries.end_index = None;
                            entries.start_area_with_offset(offset);
                        } else {
                            if entries.start_index.is_none() {
                                entries.start_area_with_offset(offset)
                            } else {
                                entries.end_area_with_offset(offset)
                            }
                        };
                    };
                    show_grid(&grid, &entries.clone());
                    window.set_title(Some(&entries.clone().show_status(offset)));
                }));
                image.add_controller(select_gesture);

                let view_gesture = gtk::GestureClick::new();
                view_gesture.set_button(1);

                view_gesture.connect_pressed(clone!(@strong entries_rc, @strong grid, @strong view, @strong stack, @strong view_scrolled_window, @strong grid_scrolled_window, @strong window => move |_,_, _, _| {
                    let entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                    if entries.grid_size == 1 { return };
                    let offset = col * grid_size + row;
                    stack.set_visible_child(&view_scrolled_window);
                    show_view(&view, &entries, offset);
                    window.set_title(Some(&entries.clone().show_status(offset)));
                }));
                image.add_controller(view_gesture);

                let motion_controller = EventControllerMotion::new(); 
                motion_controller.connect_enter(clone!(@strong entries_rc, @strong offset_rc, @strong window => move |_,_,_| {
                    if let Ok(entries) = entries_rc.try_borrow() {
                        let mut offset: RefMut<'_,usize> = offset_rc.borrow_mut();
                        *offset = col * grid_size + row;
                        window.set_title(Some(&entries.clone().show_status(*offset)));
                    } else {
                    }
                }));

                image.add_controller(motion_controller)
            }
        }
        grid_scrolled_window.set_child(Some(&grid));

        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(clone!(@strong entries_rc, @strong offset_rc, @strong grid, @strong window => move |_, key, _, _| {
            let step = 100;
            let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
            if let Some(s) = key.name() {
                match s.as_str() {
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let digit:usize = s.parse().unwrap();
                        entries.add_digit_to_resiter(digit);
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "BackSpace" => {
                        entries.remove_digit_to_register();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "g" => {
                        entries.go_to_register();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "j" => {
                        for _ in 0..10 {
                            entries.next()
                        }
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "b" => {
                        for _ in 0..10 {
                            entries.prev()
                        }
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "f" => {
                        if (entries.clone().max_cells) == 1 {
                            entries.toggle_real_size();
                            show_grid(&grid, &entries.clone());
                            window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                        }
                    },
                    "z" => {
                        entries.jump(0);
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    }
                    "n" => {
                        entries.next();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    }
                    "p" => {
                        entries.prev();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    }
                    "q"|"Escape" => {
                        entries.save_marked_file_lists(args.thumbnails);
                        window.close();
                    },
                    "r" => {
                        entries.random();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "s" => {
                        entries.toggle_to_select_with_offset(0);
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "a" => {
                        if args.thumbnails {
                            let offset: Ref<'_, usize> = offset_rc.borrow();
                            entries.start_area_with_offset(*offset);
                        } else {
                            entries.start_area();
                        };
                        show_grid(&grid, &entries.clone());
                    },
                    "e" => {
                        if args.thumbnails && stack.visible_child().unwrap() == grid_scrolled_window {
                            let offset: Ref<'_, usize> = offset_rc.borrow();
                            entries.end_area_with_offset(*offset);
                        } else {
                            entries.end_area();
                        }
                        show_grid(&grid, &entries.clone());
                    },
                    "u" => {
                        entries.reset_grid_select();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "U" => {
                        entries.reset_all_select();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "period" => {
                        if stack.visible_child().unwrap() == grid_scrolled_window {
                            let offset: Ref<'_,usize> = offset_rc.borrow();
                            stack.set_visible_child(&view_scrolled_window);
                            show_view(&view, &entries, *offset);

                        } else {
                            stack.set_visible_child(&grid_scrolled_window);
                        }
                    },
                    "space" => { 
                        if let Some(_) = args.ordered { 
                            entries.next()
                        } else {
                            entries.random()
                        }
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "Right" => {
                        let h_adj = window
                            .child()
                            .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.hadjustment()))
                            .expect("Failed to get hadjustment");
                        h_adj.set_value(h_adj.value() + step as f64);
                    },
                    "Left" => {
                        let h_adj = window
                            .child()
                            .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.hadjustment()))
                            .expect("Failed to get hadjustment");
                        h_adj.set_value(h_adj.value() - step as f64);
                    },
                    "Down" => {
                        // Scroll down
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() + step as f64);
                    },
                    "Up" => {
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() - step as f64);
                    }
                    s => { println!("{} ?", s) },
                };
                gtk::Inhibit(false)
            }
            else {
                gtk::Inhibit(false)
            };
            gtk::Inhibit(false)
        }));

        window.add_controller(evk);
        // show the first file
        if let Some(_) = args.ordered {
            let entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
            show_grid(&grid, &entries);
            window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
        } else {
            let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
            entries.random();
            show_grid(&grid, &entries);
            window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
        }

        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong entries_rc, @strong grid, @strong entry_list_rc, @strong window => move | | { 
                let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                if let Some(_) = args.ordered { 
                    entries.next();
                } else {
                    entries.random();
                };
                show_grid(&grid, &entries.clone());
                window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                Continue(true) 
            }));
        };
        window.present();
    }));
    application.set_accels_for_action("win.save", &["s"]);
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn show_grid(grid: &Grid, entries: &Entries) {
    let max_cells = entries.clone().max_cells;
    let grid_size = entries.clone().grid_size;
    for cell_index in 0 .. max_cells {
        let row = (cell_index / grid_size) as i32;
        let col = (cell_index % grid_size) as i32;
        let picture = grid.child_at(col,row).unwrap().downcast::<gtk::Picture>().unwrap();
        let offset = row as usize * grid_size + col as usize;
        let entry = entries.clone().offset_entry(offset);
        let opacity = if entry.to_select { 0.50 } else { 1.0 };
        picture.set_opacity(opacity);
        let filename = entry.file_path;
        picture.set_can_shrink(!entries.clone().real_size);
        picture.set_filename(Some(filename.clone()));
    }
}

fn show_view(grid: &Grid, entries: &Entries, offset: usize) {
    let entry = entries.clone().offset_entry(offset);
    let file_path = remove_thumb_suffix(&entry.file_path);
    let picture = grid.child_at(0,0).unwrap().downcast::<gtk::Picture>().unwrap();
    picture.set_filename(Some(file_path));
}
