use crate::navigator::Navigator;
use crate::entry::{EntryList, make_entry};
use crate::rank::Rank;
use std::time::SystemTime;
use chrono::DateTime;
use crate::Entry;

#[derive(Debug)]
pub struct Repository {
    pub entry_list: EntryList,
    pub navigator: Navigator,
    select_start: Option<usize>,
}

impl Repository {
    pub fn from_entries(entries: EntryList, cells_per_row: usize) -> Self {
        Repository{
            entry_list: entries.clone(),
            navigator: Navigator::new(entries.len() as i32, cells_per_row as i32),
            select_start: None,
        }
    }

    pub fn current_entry(&self) -> Option<&Entry> {
        Some(&self.entry_list[self.navigator.index()])
    }

    pub fn toggle_select(&mut self) {
        assert!(self.entry_list.len() > 0);
        let index = self.navigator.index();
        self.entry_list[index].to_select = !self.entry_list[index].to_select
    }

    pub fn select_point(&mut self) {
        let index = self.navigator.index();
        if self.entry_list[index].to_select {
            return
        } else {
            match self.select_start {
                None => self.select_start = Some(index),
                Some(mut other) => {
                    let (start,end) = if other <= index { (other,index) } else { (index,other) };
                    for i in start..end+1 {
                        self.entry_list[i].to_select = true
                    }
                    self.select_start = None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::Ref;
    use std::cell::RefCell;
    use std::cell::RefMut;

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
    fn after_creation_the_current_entry_is_the_first_entry() {
        let files = example_entry_list();
        let repository = Repository::from_entries(files.clone(), 2);
        assert_eq!(4, repository.navigator.capacity());
        assert_eq!(2, repository.navigator.cells_per_row());
        let entry: &Entry = repository.current_entry().unwrap();
        assert_eq!(files.clone()[0], *entry);
    }

    #[test]
    fn after_moving_one_col_current_entry_is_the_second_entry() {
        let files = example_entry_list();
        let mut repository = Repository::from_entries(files.clone(), 2);
        repository.navigator.move_rel((1,0));
        let entry: &Entry = repository.current_entry().unwrap();
        assert_eq!(files.clone()[1], *entry);
    }

    #[test]
    fn after_toggle_select_current_entry_is_selected_or_unselected() {
        let files = example_entry_list();
        // to share a mutable reference on repository
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(files.clone(), 2)));
        {
            // first mutation occurs in this scope
            let mut repository: RefMut<'_, Repository> = repository_rc.borrow_mut();
            repository.toggle_select();
            let entry: &Entry = repository.current_entry().unwrap();
            assert_eq!(true, entry.to_select);
        } // reference is released here
        // second mutation occurs in that scope
        let mut repository: RefMut<'_, Repository> = repository_rc.borrow_mut();
        repository.toggle_select();
        let entry: &Entry = repository.current_entry().unwrap();
        assert_eq!(false, entry.to_select);
    }

    #[test]
    fn after_two_select_points_a_group_of_entries_is_selected() {
        let files = example_entry_list();
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(files.clone(), 2)));
        { repository_rc.borrow_mut().navigator.move_rel((0,1)) }; // now current entry is #2 
        { repository_rc.borrow_mut().select_point() };
        { repository_rc.borrow_mut().navigator.move_rel((0,-1)) }; // now current entry is #0
        { repository_rc.borrow_mut().select_point() }; // only entries 0,1,2 are selected
        let repository: Ref<'_, Repository> = repository_rc.borrow();
        for entry in &repository.entry_list[0..3] {
            assert_eq!(true, entry.to_select)
        };
        assert_eq!(false, repository.entry_list[3].to_select)
    }
}

// at the root of the path, store a json of a hashmap file_path -> entry
// if a file_path is not in the hashmap, a) create the thumb file, count the colors, set rank to
// nostar b) insert the entry in the hashmap  c) before quitting, save the hashmap
