use std::rc::Rc;
use std::time::SystemTime;
use crate::rank::Rank;
use crate::image_data::ImageData;
use crate::paths::{thumbnail_file_path, image_data_file_path, original_file_name, original_file_path};

pub type EntryList = Vec<Entry>;

#[derive(PartialEq,Clone, Debug)]
pub struct Entry {
    pub file_path: Rc<String>,
    pub file_size: u64,
    pub modified_time: SystemTime,
    pub image_data: ImageData,
    pub delete: bool,
}


pub fn make_entry(file_path:String, file_size:u64, colors:usize, modified_time:SystemTime, initial_rank: Rank) -> Entry {
    return Entry { 
        file_path: Rc::new(file_path),
        file_size: file_size,
        image_data: ImageData {
            colors: colors,
            rank: initial_rank,
            selected: false,
            palette: [0;9],
            label_length: 0,
            label: ['\0';16],
        },
        modified_time: modified_time,
        delete: false,
    }
}


impl Entry {
    pub fn title_display(self) -> String {
        format!("{} {} [{} {} {}] {}",
            self.original_file_name(),
            if self.image_data.selected { "△" } else { "" },
            self.file_size,
            self.image_data.colors,
            self.image_data.rank.show(),
            if self.delete { "🗑" } else { ""})
    }
    pub fn label_display(&self, has_focus: bool) -> String {
        format!("{}{}{}{}{}",
            if has_focus { "▄" } else { "" },
            self.image_data.rank.show(),
            if self.image_data.selected { "△" } else { "" },
            if self.image_data.label_length > 0 {
                format!("{}", self.image_data.label.iter().collect::<String>())
            } else { String::from("") } ,
            if self.delete { "🗑" } else { "" })
    }

    pub fn record_label(&mut self, label_length: usize, label: &[char;16]) {
        for i in 0..16 {
            self.image_data.label[i] = label[i]
        };
        self.image_data.label_length = label_length;
    }

    pub fn toggle_select(&mut self) {
        self.image_data.selected = !self.image_data.selected
    }

    pub fn set_select(&mut self, value: bool) {
        self.image_data.selected = value
    }

    pub fn is_selected(&self) -> bool {
        self.image_data.selected
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
    fn title_show_the_entry_information() {
        let mut entry = make_entry(String::from("photos/foo.jpeg"), 65636, 256, a_day(), Rank::ThreeStars);
        entry.image_data.selected = true;
        assert_eq!("foo.jpeg △ [65636 256 ☆☆☆]", entry.title_display());
    }

    #[test]
    fn label_show_basic_entry_information() {
        let mut entry = make_entry(String::from("photos/foo.jpeg"), 65636, 256, a_day(), Rank::ThreeStars);
        let without_focus = false;
        let with_focus = true;
        assert_eq!("☆☆☆", entry.label_display(without_focus));
        assert_eq!("▄☆☆☆", entry.label_display(with_focus));
        entry.image_data.selected = true;
        assert_eq!("☆☆☆△", entry.label_display(without_focus));
        assert_eq!("▄☆☆☆△", entry.label_display(with_focus));
    }

}
