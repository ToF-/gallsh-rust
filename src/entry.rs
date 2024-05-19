use std::time::SystemTime;
use std::path::{PathBuf};

pub const THUMB_SUFFIX: &str = "THUMB";

pub type EntryList = Vec<Entry>;

#[derive(Clone, Debug)]
pub struct Entry {
    pub file_path: String,
    pub file_size: u64,
    pub color_size: usize,
    pub modified_time: SystemTime,
    pub to_select: bool,
}


pub fn make_entry(s:String, l:u64, c:usize, t:SystemTime) -> Entry {
    return Entry { 
        file_path: s.clone(),
        file_size: l,
        color_size: c,
        modified_time: t,
        to_select: false,
    }
}

impl Entry {
    pub fn show_status(self,) -> String {
        format!("{} {} [{}]",
            self.file_path,
            if self.to_select { "△" } else { "" },
            self.file_size)
    }

    pub fn thumbnail_file_path(self) -> String {
        if self.file_path.contains(&THUMB_SUFFIX) {
            self.file_path
        } else {
            let path = PathBuf::from(self.file_path);
            let parent = path.parent().unwrap();
            let extension = path.extension().unwrap();
            let file_stem = path.file_stem().unwrap();
            let new_file_name = format!("{}{}.{}", file_stem.to_str().unwrap(), THUMB_SUFFIX, extension.to_str().unwrap());
            let new_path = parent.join(new_file_name);
            new_path.to_str().unwrap().to_string()
        }
    }

    pub fn original_file_path(self) -> String {
        if !self.file_path.contains(THUMB_SUFFIX) {
            self.file_path
        } else {
            let path = PathBuf::from(self.file_path);
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
}
