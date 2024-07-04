use std::io;
use std::path::Path;
use std::path::Component;
use std::path::{PathBuf};
use std::fs;
use std::env;
use std::io::{Result, Error, ErrorKind};

pub const THUMB_SUFFIX: &str = "THUMB";
pub const IMAGE_DATA: &str = "IMAGE_DATA";

const DEFAULT_DIR :&str    = "images/";
const DIR_ENV_VAR :&str    = "GALLSHDIR";

pub fn is_valid_directory(dir: &str) -> bool {
    let path = PathBuf::from(dir);
    if ! path.exists() {
       return false
    } else {
        if let Ok(metadata) = fs::metadata(path) {
            return metadata.is_dir()
        } else {
            return false
        }
    }
}

pub fn check_path(dir: &str, confirm_create: bool) -> Result<PathBuf> {
    let path = PathBuf::from(dir);
    if !path.exists() {
        if confirm_create {
            println!("directory {} doesn't exist. Create ?", dir);
            let mut response = String::new();
            let stdin = io::stdin();
            stdin.read_line(&mut response).expect("can't read from stdin");
            match response.chars().next() {
                Some(c) => {
                    if c == 'y' || c == 'Y' {
                        match fs::create_dir(path.clone()) {
                            Ok(()) => Ok(path),
                            Err(err) => return Err(err),
                        }
                    } else {
                        Err(Error::new(ErrorKind::Other, "directory creation cancelled"))
                    }
                },
                None => Err(Error::new(ErrorKind::Other, "directory creation cancelled")),
            }
        } else {
            Err(Error::new(ErrorKind::Other, format!("path {} doesn't exist", dir)))
        }
    } else {
        if is_valid_directory(dir) {
            Ok(path)
        } else {
            Err(Error::new(ErrorKind::Other, format!("path {} is not a directory", dir)))
        }
    }
}

pub fn check_label_path(dir: &str, label: &str) -> Result<PathBuf> {
    let path = PathBuf::from(dir).join(label);
    check_path(path.to_str().unwrap(),true)
}

pub fn determine_path(directory: Option<String>) -> String {
    let gallshdir = env::var(DIR_ENV_VAR);
    if let Some(directory_arg) = directory {
        String::from(directory_arg)
    } else if let Ok(standard_dir) = &gallshdir {
        String::from(standard_dir)
    } else {
        println!("GALLSHDIR variable not set. Using {} as default.", DEFAULT_DIR);
        String::from(DEFAULT_DIR)
    }
}
pub fn thumbnail_file_path(file_path: &str) -> String {
    if file_path.contains(&THUMB_SUFFIX) {
        file_path.to_string()
    } else {
        let path = PathBuf::from(file_path);
        let parent = path.parent().unwrap();
        let extension = path.extension().unwrap();
        let file_stem = path.file_stem().unwrap();
        let new_file_name = format!("{}{}.{}", file_stem.to_str().unwrap(), THUMB_SUFFIX, extension.to_str().unwrap());
        let new_path = parent.join(new_file_name);
        new_path.to_str().unwrap().to_string()
    }
}

pub fn image_data_file_path(file_path: &str) -> String {
    let image_file_path = original_file_path(file_path);
    let path = PathBuf::from(image_file_path);
    let parent = path.parent().unwrap();
    let file_stem = path.file_stem().unwrap().to_str().unwrap();
    let new_file_name = format!("{}{}.json", file_stem, IMAGE_DATA);
    let new_path = parent.join(new_file_name);
    new_path.to_str().unwrap().to_string()
}

pub fn directory(file_path: &str) -> String {
    let path = PathBuf::from(file_path);
    let parent = path.parent()
        .expect(&format!("file path {} has no parent",file_path));
    let directory = parent.components().next_back()
        .expect(&format!("can't find directory of {}", file_path));
    <Component<'_> as AsRef<Path>>::as_ref(&directory).to_str().unwrap().to_string()
}

pub fn original_file_path(file_path: &str) -> String {
    if !is_thumbnail(file_path) {
        file_path.to_string()
    } else {
        let path = PathBuf::from(file_path);
        let parent = path.parent().unwrap();
        let extension = path.extension().unwrap();
        let file_stem = path.file_stem().unwrap().to_str().unwrap();
        let new_file_stem = match file_stem.strip_suffix("THUMB") {
            Some(s) => s,
            None => &file_stem,
        };
        let new_file_name = format!("{}.{}", new_file_stem, extension.to_str().unwrap());
        let new_path = parent.join(new_file_name);
        new_path.to_str().unwrap().to_string()
    }
}

pub fn original_file_name(file_path: &str) -> String  {
    let original = original_file_path(file_path);
    let path = PathBuf::from(original);
    path.file_name().unwrap().to_str().unwrap().to_string()
}

pub fn is_thumbnail(file_path: &str) -> bool {
    file_path.contains(&THUMB_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_file_path_is_file_path_with_an_add_thumb_suffix() {
        assert_eq!("photos/fooTHUMB.jpeg", thumbnail_file_path("photos/foo.jpeg"));
    }
    #[test]
    fn original_file_name_is_rid_of_any_thumb_suffix_and_path() {
        assert_eq!("foo.jpeg", original_file_name("photos/fooTHUMB.jpeg"));
    }

    #[test]
    fn thumbnail_file_path_is_added_the_thumb_suffix() {
        assert_eq!("photos/fooTHUMB.jpeg", thumbnail_file_path("photos/foo.jpeg"));
    }

    #[test]
    fn image_data_file_path_is_added_the_image_data_suffix_and_json_extension() {
        assert_eq!("photos/fooIMAGE_DATA.json", image_data_file_path("photos/foo.jpeg"));
    }

}
