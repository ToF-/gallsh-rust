use std::path::PathBuf;

pub const THUMB_SUFFIX: &str = "THUMB";
pub const IMAGE_DATA: &str = "IMAGE_DATA";


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

pub fn original_file_path(file_path: &str) -> String {
    if !file_path.contains(&THUMB_SUFFIX) {
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
