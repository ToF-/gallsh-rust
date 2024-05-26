use crate::navigator::Navigator;
use crate::entry::{EntryList, make_entry};
use crate::rank::Rank;
use std::time::SystemTime;

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

    #[test]
    fn creating_a_repository_from_a_list_of_entries() {
        let files: EntryList = vec!(
            make_entry(String::from("foo.jpeg"), 100, 5, SystemTime::now(), Rank::NoStar),
            make_entry(String::from("bar.jpeg"), 1000, 15, SystemTime::now(), Rank::ThreeStars),
            make_entry(String::from("qux.jpeg"), 10, 25, SystemTime::now(), Rank::TwoStars),
        );

        let repository = Repository::from_entries(files, 1);
        assert_eq!(3, repository.navigator.capacity());
    }
}
