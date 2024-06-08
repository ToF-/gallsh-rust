use crate::image_data::ImageData;
use std::fs::remove_file;
use crate::paths::is_thumbnail;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::PathBuf;
use crate::THUMB_SUFFIX;
use crate::make_entry;
use std::fs;
use regex::Regex;
use walkdir::WalkDir;
use crate::EntryList;
use crate::Rank;
use crate::image::get_image_color;
use std::fs::read_to_string;
use crate::Entry;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use thumbnailer::error::ThumbResult;
use thumbnailer::create_thumbnails;
use thumbnailer::ThumbnailSize;
use std::ffi::OsStr;
use std::collections::HashSet;

const VALID_EXTENSIONS: [&'static str; 6] = ["jpg", "jpeg", "png", "JPG", "JPEG", "PNG"];
const SELECTION_FILE_NAME: &str = "selections";

use std::io::{Result,Error, ErrorKind};

pub fn set_original_picture_file(picture: &gtk::Picture, entry: &Entry) -> Result<()> {
    let original = entry.original_file_path();
    let path = Path::new(&original);
    if path.exists() {
        picture.set_filename(Some(original));
        Ok(())
    } else {
        Err(Error::new(ErrorKind::Other, format!("file {} doesn't exist", original)))
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

fn create_thumbnail(entry: &Entry) -> Result<()> {
    let original = entry.original_file_path();
    let thumbnail = entry.thumbnail_file_path();
    println!("creating thumbnail {}", thumbnail);
    match File::open(original.clone()) {
        Err(err) => Err(err),
        Ok(input_file) => {
            let source_path = Path::new(&original);
            let extension = match source_path.extension()
                .and_then(OsStr::to_str) { 
                    None => return Err(Error::new(ErrorKind::Other, format!("source file has no extension"))),
                    Some(ext) => ext,
                };
            let reader = BufReader::new(input_file);
            let output_file = match File::create(thumbnail) {
                Err(err) => return Err(err),
                Ok(file) => file,
            };
            match write_thumbnail(reader, extension, output_file) {
                Err(err) => Err(Error::new(ErrorKind::Other, err)),
                Ok(_) => Ok (()),
            }
        },
    }
}

pub fn set_thumbnail_picture_file(picture: &gtk::Picture, entry: &Entry) -> Result<()> {
    let thumbnail = entry.thumbnail_file_path();
    let path = Path::new(&thumbnail);
    if path.exists() {
        picture.set_filename(Some(thumbnail));
        Ok(())
    } else {
        match create_thumbnail(entry) {
            Ok(()) => {
                picture.set_filename(Some(thumbnail));
                Ok(())
            },
            err => err,
        }
    }
}

pub fn set_image_data(entry: &mut Entry) -> Result<()> {
    let image_data = entry.image_data_file_path();
    let path = Path::new(&image_data);
    if path.exists() {
        match read_to_string(path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(image_data) => {
                    entry.image_data = image_data;
                    Ok(())
                },
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err),
        }
    } else {
        match get_image_color(&entry.original_file_path()) {
            Ok(colors) => {
                entry.image_data = ImageData {
                    colors: colors,
                    rank: Rank::NoStar,
                    selected: false,
                };
                let _ = save_image_data(&entry);
                Ok(())
            },
            Err(err) => Err(Error::new(ErrorKind::Other,err)),
        }
    }
}

pub fn save_image_data(entry: &Entry) -> Result<()> {
    let image_data_file_path = entry.image_data_file_path();
    let path = Path::new(&image_data_file_path);
    match File::create(path) {
        Ok(file) => {
            match serde_json::to_writer(file, &entry.image_data) {
                Ok(_) => Ok(()),
                Err(err) => Err(err.into()),
            }
        },
        Err(err) => {
            println!("error saving image data {} : {}", path.display(), err);
            Err(err)
        },
    }
}

pub fn save_image_list(list: Vec<String>) {
    if let Ok(mut file) = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(SELECTION_FILE_NAME) {
        for line in list {
            writeln!(file, "{}", line).expect("can't write")
        }
    }
}

pub fn delete_selection_file() {
    let path = Path::new(SELECTION_FILE_NAME);
    if path.exists() {
        let _ = remove_file(path);
    }
}

fn push_entry_from_path(path: &Path, pattern_opt: Option<String>, entry_list: &mut EntryList) -> Result<()> {
    let valid_extension = match path.extension() {
        Some(extension) => VALID_EXTENSIONS.contains(&extension.to_str().unwrap()),
        None => false,
    };
    let matches_pattern = path.is_file() && match pattern_opt {
        None => true,
        Some(pattern) => {
            match Regex::new(&pattern) {
                Ok(reg_exp) => match reg_exp.captures(path.to_str().unwrap()) {
                    Some(_) => true,
                    None => false,
                },
                Err(err) => {
                    println!("can't parse regular expression {}: {}", pattern, err);
                    false
                },
            }
        },
    };
    let not_a_thumbnail = match path.to_str().map(|filename| filename.contains(THUMB_SUFFIX)) {
        Some(false) => true,
        _ => false,
    };
    if valid_extension && not_a_thumbnail && matches_pattern {
        if let Ok(metadata) = fs::metadata(&path) {
            let file_size = metadata.len();
            if file_size == 0 {
                println!("file {} has a size of 0", path.display())
            };
            let modified_time = metadata.modified().unwrap();
            let name = path.to_str().unwrap().to_string().to_owned();
            let mut entry = make_entry(name, file_size, 0, modified_time, Rank::NoStar);
            match set_image_data(&mut entry) {
                Ok(_) => {
                    entry_list.push(entry);
                },
                Err(_) => {
                    println!("can't find or create image data for file {}", path.display())
                },
            }
        } else {
            println!("can't open: {}", path.display());
        }
    };
    Ok(())
}
pub fn entries_from_directory(dir: &str, pattern_opt: Option<String>) -> Result<EntryList> {
    let mut entry_list: EntryList = Vec::new();
    for path in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()).map(|e| e.into_path()) {
        push_entry_from_path(&path, pattern_opt.clone(), &mut entry_list).unwrap();
    };
    Ok(entry_list.clone())
}

pub fn entries_from_file(file: &str) -> Result<EntryList> {
    let mut entry_list: EntryList = Vec::new();
    let path = PathBuf::from(file);
    push_entry_from_path(&path, None, &mut entry_list).unwrap();
    Ok(entry_list.clone())
}

pub fn entries_from_reading_list(reading_list: &str, pattern_opt: Option<String>) -> Result<EntryList> {
    match read_to_string(reading_list) {
        Err(err) => {
            Err(err)
        },
        Ok(content) => {
            let mut entry_list: EntryList = Vec::new();
            let mut file_paths_set: HashSet<String> = HashSet::new();
            for path in content.lines().map(String::from).filter(|p| !is_thumbnail(p)).collect::<Vec<_>>().into_iter().map(|line| PathBuf::from(line)) {
                let file_path = path.to_str().unwrap().to_string();
                if ! file_paths_set.contains(&file_path) {
                    file_paths_set.insert(file_path);
                    push_entry_from_path(&path, pattern_opt.clone(), &mut entry_list).unwrap()
                } else {
                    println!("{} already in reading list", path.display());
                }
            };
            Ok(entry_list.clone())
        },
    }
}

fn copy_file_to_target_directory(file_path: &Path, target_directory: &Path) -> Result<u64> {
    let file_name = file_path.file_name().unwrap();
    let target_file_path = target_directory.join(file_name);
    println!("copy {} to {}", file_path.display(), target_file_path.display());
    std::fs::copy(file_path, target_file_path)
}

pub fn copy_entry(entry: &Entry, target_path: &Path) {
    let file_name = entry.original_file_path();
    let thumbnail_name = entry.thumbnail_file_path();
    let image_data_name = entry.image_data_file_path();
    let file_path = Path::new(&file_name);
    let thumbnail_path = Path::new(&thumbnail_name);
    let image_data_path = Path::new(&image_data_name);
    match copy_file_to_target_directory(file_path, target_path)
        .and_then(|_| copy_file_to_target_directory(thumbnail_path, target_path))
        .and_then(|_| copy_file_to_target_directory(image_data_path, target_path)) {
            Ok(_) => {},
            Err(err) => println!("error: {}", err),
        }
}

pub fn delete_entry(entry: &Entry) {
    let file_name = entry.original_file_path();
    let thumbnail_name = entry.thumbnail_file_path();
    let image_data_name = entry.image_data_file_path();
    let file_path = Path::new(&file_name);
    let thumbnail_path = Path::new(&thumbnail_name);
    let image_data_path = Path::new(&image_data_name);
    if file_path.exists() {
        let _ = remove_file(file_path);
    };
    if thumbnail_path.exists() {
        let _ = remove_file(thumbnail_path);
    };
    if image_data_path.exists() {
        let _ = remove_file(image_data_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn can_read_entries_from_a_directory_without_reading_the_thumbnails() {
        let entries = entries_from_directory("./testdata", None).unwrap();
        assert_eq!(7, entries.len());
        let index = entries.iter().position(|e| e.original_file_name() == "UN_Fight_for_Freedom_Leslie_Ragan_1943_poster_-_restoration1.jpeg").unwrap();
        assert_eq!(56984, entries[index].image_data.colors);
        assert_eq!(67293, entries[index].file_size);
    }

    #[test]
    fn can_read_entries_from_a_directory_with_pattern() {
        let entries = entries_from_directory("./testdata", Some(String::from("1.*4"))).unwrap();
        assert_eq!(3, entries.len());
    }

    #[test]
    fn can_read_entries_from_reading_list() {
        let entries = entries_from_reading_list("./testdata/reading_list", None).unwrap();
        assert_eq!(4, entries.len());
        assert_eq!("020_African_blue_flycatcher_at_Kibale_forest_National_Park_Photo_by_Giles_Laurent.jpeg", entries[0].original_file_name());
        assert_eq!("Continental_I-1430_NASM.jpg", entries[1].original_file_name());
        assert_eq!("DAN-13-Danzig-100_Mark_(1922).jpg", entries[2].original_file_name());
        assert_eq!("Johannes_Vermeer_-_Lady_at_the_Virginal_with_a_Gentleman,_'The_Music_Lesson'_-_Google_Art_Project.jpg", entries[3].original_file_name());
    }

    #[test]
    fn can_read_entry_for_a_file() {
        let entries = entries_from_file("./testdata/020_African_blue_flycatcher_at_Kibale_forest_National_Park_Photo_by_Giles_Laurent.jpeg").unwrap();
        assert_eq!(1, entries.len());
        assert_eq!("020_African_blue_flycatcher_at_Kibale_forest_National_Park_Photo_by_Giles_Laurent.jpeg", entries[0].original_file_name());
    }
}

