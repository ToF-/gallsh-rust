use crate::navigator::Navigator;
use crate::entry::{EntryList, make_entry};
use crate::rank::Rank;
use std::time::SystemTime;
use chrono::DateTime;
use crate::Entry;
use crate::Order;
use rand::thread_rng;
use std::cmp::Ordering::Equal;
use rand::prelude::SliceRandom;

pub struct Repository {
    pub entry_list: EntryList,
    pub navigator: Navigator,
    select_start: Option<usize>,
    order: Option<Order>,
}

impl Repository {
    pub fn from_entries(entries: EntryList, cells_per_row: usize) -> Self {
        Repository{
            entry_list: entries.clone(),
            navigator: Navigator::new(entries.len() as i32, cells_per_row as i32),
            select_start: None,
            order: Some(Order::Random),
        }
    }

    pub fn sort_by(&mut self, order: Order) {
        match order {
            Order::Colors => self.entry_list.sort_by(|a, b| { 
                let cmp = (a.colors).cmp(&b.colors);
                if cmp == Equal {
                    a.file_path.cmp(&b.file_path)
                } else {
                    cmp
                }
            }),
            Order::Date => self.entry_list.sort_by(|a, b| { a.modified_time.cmp(&b.modified_time) }),
            Order::Name => self.entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) }),
            Order::Size => self.entry_list.sort_by(|a, b| { a.file_size.cmp(&b.file_size) }),
            Order::Value => self.entry_list.sort_by(|a,b| {
                let cmp = (a.rank as usize).cmp(&(b.rank as usize));
                if cmp == Equal {
                    a.file_path.cmp(&b.file_path)
                } else {
                    cmp
                }
            }),
            Order::Random => self.entry_list.shuffle(&mut thread_rng()),
        };
        self.order = Some(order)
    }

    pub fn slice(&mut self, low_index: Option<usize>, high_index: Option<usize>) {
        let start = match low_index {
            None => 0,
            Some(index) => index,
        };
        let end = match high_index {
            None => self.entry_list.len(),
            Some(index) => index + 1,
        };
        self.entry_list = self.entry_list.clone()[start..end].to_vec();
        self.navigator = Navigator::new(self.entry_list.len() as i32, self.navigator.cells_per_row());
        self.select_start = None
    }

    pub fn current_entry(&self) -> Option<&Entry> {
        Some(&self.entry_list[self.navigator.index()])
    }

    pub fn toggle_select(&mut self) {
        assert!(self.entry_list.len() > 0);
        let index = self.navigator.index();
        self.entry_list[index].to_select = !self.entry_list[index].to_select
    }

    pub fn set_rank(&mut self, rank: Rank) {
        assert!(self.entry_list.len() > 0);
        let index = self.navigator.index();
        self.entry_list[index].rank = rank
    }

    pub fn select_point(&mut self) {
        let index = self.navigator.index();
        if self.entry_list[index].to_select {
            return
        } else {
            match self.select_start {
                None => self.select_start = Some(index),
                Some(other) => {
                    let (start,end) = if other <= index { (other,index) } else { (index,other) };
                    for i in start..end+1 {
                        self.entry_list[i].to_select = true
                    }
                    self.select_start = None
                }
            }
        }
    }

    pub fn rank_point(&mut self, rank: Rank) {
        let index = self.navigator.index();
        if self.entry_list[index].rank == rank {
            return
        } else {
            match self.select_start {
                None => self.select_start = Some(index),
                Some(other) => {
                    let (start,end) = if other <= index { (other,index) } else { (index,other) };
                    for i in start..end+1 {
                        self.entry_list[i].rank = rank
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
    use std::cell::RefCell;
    use std::cell::RefMut;

    fn example() -> EntryList {
        let day_a: SystemTime = DateTime::parse_from_rfc2822("Sun, 1 Jan 2023 10:52:37 GMT").unwrap().into();
        let day_b: SystemTime = DateTime::parse_from_rfc2822("Sat, 1 Jul 2023 10:52:37 GMT").unwrap().into();
        let day_c: SystemTime = DateTime::parse_from_rfc2822("Mon, 1 Jan 2024 10:52:37 GMT").unwrap().into();
        let day_d: SystemTime = DateTime::parse_from_rfc2822("Mon, 1 Jan 2024 11:52:37 GMT").unwrap().into();
        vec!(
            make_entry(String::from("photos/foo.jpeg"), 100, 5, day_d, Rank::NoStar),
            make_entry(String::from("photos/bar.jpeg"), 1000, 15, day_b, Rank::ThreeStars),
            make_entry(String::from("photos/qux.jpeg"), 10, 25, day_c, Rank::TwoStars),
            make_entry(String::from("photos/bub.jpeg"), 100, 25, day_a, Rank::OneStar))
    }

    #[test]
    fn after_creation_the_current_entry_is_the_first_entry() {
        let repository = Repository::from_entries(example().clone(), 2);
        assert_eq!(4, repository.navigator.capacity());
        assert_eq!(2, repository.navigator.cells_per_row());
        let entry: &Entry = repository.current_entry().unwrap();
        assert_eq!(example().clone()[0], *entry);
    }

    #[test]
    fn after_moving_one_col_current_entry_is_the_second_entry() {
        let mut repository = Repository::from_entries(example().clone(), 2);
        repository.navigator.move_rel((1,0));
        let entry: &Entry = repository.current_entry().unwrap();
        assert_eq!(example().clone()[1], *entry);
    }

    #[test]
    fn after_toggle_select_current_entry_is_selected_or_unselected() {
        // to share a mutable reference on repository
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
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
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().navigator.move_rel((0,1)) }; // now current entry is #2 
        { repository_rc.borrow_mut().select_point() };
        { repository_rc.borrow_mut().navigator.move_rel((0,-1)) }; // now current entry is #0
        { repository_rc.borrow_mut().select_point() }; // only entries 0,1,2 are selected
        let repository = repository_rc.borrow();
        for entry in &repository.entry_list[0..3] {
            assert_eq!(true, entry.to_select)
        };
        assert_eq!(false, repository.entry_list[3].to_select)
    }

    #[test]
    fn after_setting_rank_current_entries_has_a_new_rank() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().set_rank(Rank::ThreeStars) };
        let repository = repository_rc.borrow();
        assert_eq!(Rank::ThreeStars, repository.current_entry().unwrap().rank);
    }

    #[test]
    fn after_two_rank_points_a_group_on_entries_has_rank_chaned() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().navigator.move_rel((0,1)) }; // now current entry is #2 
        { repository_rc.borrow_mut().select_point() };
        { repository_rc.borrow_mut().navigator.move_rel((0,-1)) }; // now current entry is #0
        { repository_rc.borrow_mut().rank_point(Rank::TwoStars) }; // only entries 0,1,2 are ranked
        let repository = repository_rc.borrow();
        for entry in &repository.entry_list[0..3] {
            assert_eq!(Rank::TwoStars, entry.rank)
        };
        assert_eq!(Rank::OneStar, repository.entry_list[3].rank)
    }

    #[test]
    fn sorting_entries_by_date() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().sort_by(Order::Date) };
        { assert_eq!(String::from("bub.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(1) };
        { assert_eq!(String::from("bar.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(2) };
        { assert_eq!(String::from("qux.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(3) };
        { assert_eq!(String::from("foo.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
    }

    #[test]
    fn sorting_entries_by_name() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().sort_by(Order::Name) };
        { assert_eq!(String::from("bar.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(1) };
        { assert_eq!(String::from("bub.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(2) };
        { assert_eq!(String::from("foo.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(3) };
        { assert_eq!(String::from("qux.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
    }
    #[test]
    fn sorting_entries_by_colors_then_name() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().sort_by(Order::Colors) };
        { assert_eq!(String::from("foo.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(1) };
        { assert_eq!(String::from("bar.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(2) };
        { assert_eq!(String::from("bub.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(3) };
        { assert_eq!(String::from("qux.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
    }
    #[test]
    fn sorting_entries_by_value_then_name() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().sort_by(Order::Value) };
        { assert_eq!(String::from("bar.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(1) };
        { assert_eq!(String::from("qux.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(2) };
        { assert_eq!(String::from("bub.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
        { repository_rc.borrow_mut().navigator.move_to_index(3) };
        { assert_eq!(String::from("foo.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name()) };
    }

    #[test]
    fn slicing_entries_without_limits_yields_the_whole_set() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().slice(None, None) };
        assert_eq!(4, repository_rc.borrow().entry_list.len());
    }
    #[test]
    fn slicing_entries_with_low_limit_yields_a_portion_of_the_set() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().slice(Some(2), None) };
        assert_eq!(2, repository_rc.borrow().entry_list.len());
        assert_eq!(String::from("qux.jpeg"), repository_rc.borrow().current_entry().unwrap().original_file_name());
    }
    #[test]
    fn slicing_entries_with_high_limit_yields_a_portion_of_the_set() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().slice(None, Some(2)) };
        assert_eq!(3, repository_rc.borrow().entry_list.len());
    }
}

