use std::rc::Rc;
use std::time::SystemTime;
use std::path::{PathBuf};

pub const THUMB_SUFFIX: &str = "THUMB";
pub const IMAGE_DATA: &str = "IMAGE_DATA";
pub const NO_STAR: usize = 3;
pub const ONE_STAR: usize = 2;
pub const TWO_STARS: usize = 1;
pub const THREE_STARS: usize = 0;

pub type EntryList = Vec<Entry>;

#[derive(Clone, Debug)]
pub struct Entry {
    pub file_path: Rc<String>,
    pub file_size: u64,
    pub colors: usize,
    pub modified_time: SystemTime,
    pub to_select: bool,
    pub initial_rank: usize,
    pub rank: usize,
}


pub fn make_entry(file_path:String, file_size:u64, colors:usize, modified_time:SystemTime, initial_rank: usize) -> Entry {
    return Entry { 
        file_path: Rc::new(file_path),
        file_size: file_size,
        colors: colors,
        modified_time: modified_time,
        to_select: false,
        initial_rank: initial_rank,
        rank: initial_rank,
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

fn show_rank(rank: usize) -> String {
    let limit = if rank > 3 { 0 } else { 3 - rank };
    if limit > 0 {
        "☆".repeat(limit)
    } else {
        "".to_string()
    }
}
impl Entry {
    pub fn show_status(self,) -> String {
        format!("{} {} [{} {} {}]",
            self.clone().original_file_path(),
            if self.to_select { "△" } else { "" },
            self.file_size,
            self.colors,
            show_rank(self.rank))
    }

    pub fn thumbnail_file_path(self) -> String {
        thumbnail_file_path(&self.file_path)
    }

    pub fn original_file_path(self) -> String {
        original_file_path(&self.file_path)
    }
}
