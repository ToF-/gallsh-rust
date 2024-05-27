use crate::navigator::Navigator;
use crate::entry::{EntryList, make_entry};
use crate::rank::Rank;
use std::time::SystemTime;
use chrono::{prelude::*, Days, Months};

pub struct Repository {
    pub entry_list: EntryList,
    pub navigator: Navigator,
}

impl Repository {
    pub fn from_entries(entries: EntryList, cells_per_row: usize) -> Self {
        let entry_list = EntryList::new();
        Repository{
            entry_list: entry_list,
            navigator: Navigator::new(entries.len() as i32, cells_per_row as i32),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn example_entry_list() -> EntryList {
        let day_a: SystemTime = DateTime::parse_from_rfc2822("Sun, 1 Jan 2023 10:52:37 GMT").unwrap().into();
        let day_b: SystemTime = DateTime::parse_from_rfc2822("Sat, 1 Jul 2023 10:52:37 GMT").unwrap().into();
        let day_c: SystemTime = DateTime::parse_from_rfc2822("Mon, 1 Jan 2024 10:52:37 GMT").unwrap().into();
        vec!(
            make_entry(String::from("photos/foo.jpeg"), 100, 5, day_a, Rank::NoStar),
            make_entry(String::from("photos/bar.jpeg"), 1000, 15, day_b, Rank::ThreeStars),
            make_entry(String::from("photos/qux.jpeg"), 10, 25, day_c, Rank::TwoStars),
            make_entry(String::from("photos/bub.jpeg"), 100, 25, day_a, Rank::OneStar))
    }
    #[test]
    fn creating_a_repository_from_a_list_of_entries() {
        let files = example_entry_list();
        let repository = Repository::from_entries(files, 1);
        assert_eq!(4, repository.navigator.capacity());
    }
}
