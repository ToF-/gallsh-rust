use std::rc::Rc;
use std::time::SystemTime;
use std::path::{PathBuf};
use crate::rank::Rank;


pub const THUMB_SUFFIX: &str = "THUMB";
pub const IMAGE_DATA: &str = "IMAGE_DATA";

pub type EntryList = Vec<Entry>;

#[derive(PartialEq,Clone, Debug)]
pub struct Entry {
    pub file_path: Rc<String>,
    pub file_size: u64,
    pub colors: usize,
    pub modified_time: SystemTime,
    pub to_select: bool,
    pub initial_rank: Rank,
    pub rank: Rank,
}


pub fn make_entry(file_path:String, file_size:u64, colors:usize, modified_time:SystemTime, initial_rank: Rank) -> Entry {
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

pub fn original_file_name(file_path: &str) -> String  {
    let original = original_file_path(file_path);
    let path = PathBuf::from(original);
    path.file_name().unwrap().to_str().unwrap().to_string()
}

impl Entry {
    pub fn show_status(self) -> String {
        format!("{} {} [{} {} {}]",
            self.original_file_name(),
            if self.to_select { "△" } else { "" },
            self.file_size,
            self.colors,
            self.rank.show())
    }
    pub fn label(&self, has_focus: bool) -> String {
        format!("{}{}{}",
            if has_focus { "▄" } else { "" },
            self.rank.show(),
            if self.to_select { "△" } else { "" })
    }

    pub fn thumbnail_file_path(&self) -> String {
        thumbnail_file_path(&self.file_path)
    }

    pub fn original_file_path(&self) -> String {
        original_file_path(&self.file_path)
    }

    pub fn original_file_name(&self) -> String {
        original_file_name(&self.file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    fn a_day() -> SystemTime {
        DateTime::parse_from_rfc2822("Sun, 1 Jan 2023 10:52:37 GMT").unwrap().into()
    }

    #[test]
    fn original_file_path_is_rid_of_any_thumb_suffix() {
            let entry = make_entry(String::from("photos/fooTHUMB.jpeg"), 100, 5, a_day(), Rank::NoStar);
            assert_eq!(String::from("photos/foo.jpeg"), entry.original_file_path());
    }

    #[test]
    fn original_file_name_is_rid_of_any_thumb_suffixi_and_path() {
            let entry = make_entry(String::from("photos/fooTHUMB.jpeg"), 100, 5, a_day(), Rank::NoStar);
            assert_eq!(String::from("foo.jpeg"), entry.original_file_name());
    }

    #[test]
    fn thumbnail_file_path_is_add_the_thumb_suffix() {
            let entry = make_entry(String::from("photos/foo.jpeg"), 100, 5, a_day(), Rank::NoStar);
            assert_eq!(String::from("photos/fooTHUMB.jpeg"), entry.thumbnail_file_path());
    }
}

