use std::rc::Rc;
use std::time::SystemTime;
use std::path::{PathBuf};
use crate::rank::Rank;
use crate::paths::{thumbnail_file_path, image_data_file_path, original_file_name,original_file_path};

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

    pub fn image_data_file_path(&self) -> String {
        image_data_file_path(&self.file_path)
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
    fn thumbnail_file_path_is_added_the_thumb_suffix() {
        let entry = make_entry(String::from("photos/foo.jpeg"), 100, 5, a_day(), Rank::NoStar);
        assert_eq!(String::from("photos/fooTHUMB.jpeg"), entry.thumbnail_file_path());
    }
    #[test]
    fn image_data_file_path_is_added_the_image_data_suffix_and_json_extension() {
        let entry = make_entry(String::from("photos/foo.jpeg"), 100, 5, a_day(), Rank::NoStar);
        assert_eq!(String::from("photos/fooIMAGE_DATA.json"), entry.image_data_file_path());
    }
}
