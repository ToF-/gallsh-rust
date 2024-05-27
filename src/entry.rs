use std::rc::Rc;
use std::time::SystemTime;
use crate::rank::Rank;
use crate::paths::{thumbnail_file_path, original_file_name,original_file_path};

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
    pub fn title_display(self) -> String {
        format!("{} {} [{} {} {}]",
            self.original_file_name(),
            if self.to_select { "△" } else { "" },
            self.file_size,
            self.colors,
            self.rank.show())
    }
    pub fn label_display(&self, has_focus: bool) -> String {
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
    fn title_show_the_entry_information() {
        let mut entry = make_entry(String::from("photos/foo.jpeg"), 65636, 256, a_day(), Rank::ThreeStars);
        entry.to_select = true;
        assert_eq!("foo.jpeg △ [65636 256 ☆☆☆]", entry.title_display());
    }

    #[test]
    fn label_show_basic_entry_information() {
        let mut entry = make_entry(String::from("photos/foo.jpeg"), 65636, 256, a_day(), Rank::ThreeStars);
        let without_focus = false;
        let with_focus = true;
        assert_eq!("☆☆☆", entry.label_display(without_focus));
        assert_eq!("▄☆☆☆", entry.label_display(with_focus));
        entry.to_select = true;
        assert_eq!("☆☆☆△", entry.label_display(without_focus));
        assert_eq!("▄☆☆☆△", entry.label_display(with_focus));
    }

}
