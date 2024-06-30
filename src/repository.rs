use std::io::{Result,Error, ErrorKind};
use crate::read_entries;
use crate::ensure_thumbnails;
use crate::Args;
use crate::Direction;
use crate::Entry;
use crate::Order;
use crate::entry::{EntryList};
use crate::navigator::Coords;
use crate::navigator::Navigator;
use crate::picture_io::copy_entry;
use crate::picture_io::delete_entry;
use crate::picture_io::delete_selection_file;
use crate::picture_io::move_entries_with_label;
use crate::picture_io::save_image_list;
use crate::picture_io;
use crate::rank::Rank;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::cmp::Ordering::Equal;
use std::cmp::min;
use std::path::Path;

pub struct Repository {
    entry_list: EntryList,
    navigator: Navigator,
    select_start: Option<usize>,
    order: Option<Order>,
    register: Option<usize>,
    real_size_on: bool,
    palette_extract_on: bool,
    max_selected: usize,
    label_edit_mode_on: bool,
    label: [char;16],
    label_length: usize,
    copy_selection_target: Option<String>,
    move_selection_target: Option<String>,
}

pub fn init_repository(args: &Args) -> Result<Repository> {
    let copy_selection_target = match args.copy_selection_target() {
        Ok(target) => target,
        Err(err) => return Err(Error::new(ErrorKind::Other, err)),
    };
    let move_selection_target = match args.move_selection_target() {
        Ok(target) => target,
        Err(err) => return Err(Error::new(ErrorKind::Other, err)),
    };
    let entry_list = match read_entries(args.reading.clone(), args.file.clone(), args.path(), args.pattern.clone()) {
        Ok(list) => list,
        Err(err) => return Err(Error::new(ErrorKind::Other, err)),
    };
    if args.update_image_data {
        ensure_thumbnails(&entry_list);
        return Err(Error::new(ErrorKind::Other, "updating thumbnails done"))
    };

    if let Some(parameters) = &args.move_label {
        let label = parameters[0].clone();
        let target = parameters[1].clone();
        match move_entries_with_label(&entry_list, &label, &target) {
            Ok(()) => return Err(Error::new(ErrorKind::Other, "move entries done")),
            Err(err) => return Err(Error::new(ErrorKind::Other, err)),
        }
    }

    let mut repository = Repository::from_entries(entry_list, args.grid_size(), copy_selection_target.clone(), move_selection_target.clone());


    repository.sort_by(args.order());
    repository.slice(args.from, args.to);

    println!("{} entries", repository.capacity());

    if let Some(index) = args.index {
        if repository.can_move_to_index(index) {
            repository.move_to_index(index)
        } else {
            return Err(Error::new(ErrorKind::Other, "entry index out of range"))
        }
    };

    if args.extraction {
        repository.toggle_palette_extract();
    };
    Ok(repository)
}
impl Repository {
    pub fn from_entries(entries: EntryList, cells_per_row: usize, copy_selection_target: Option<String>, move_selection_target: Option<String>) -> Self {
        Repository{
            entry_list: entries.clone(),
            navigator: Navigator::new(entries.len() as i32, cells_per_row as i32),
            select_start: None,
            order: Some(Order::Random),
            register: None,
            real_size_on: false,
            palette_extract_on: false,
            max_selected: entries.clone().iter().filter(|e| e.image_data.selected).count(),
            label_edit_mode_on: false,
            label: ['\0';16],
            label_length: 0,
            copy_selection_target : copy_selection_target,
            move_selection_target : move_selection_target,
        }
    }

    pub fn label_edit_mode_on(&self) -> bool {
        self.label_edit_mode_on
    }

    pub fn add_label_char(&mut self, ch: char) {
        if self.label_length < 16 {
            self.label[self.label_length] = ch;
            self.label_length += 1
        }
    }

    pub fn remove_label_char(&mut self) {
        if self.label_length > 0 {
            self.label[self.label_length-1] = '\0';
            self.label_length -= 1
        }
    }

    pub fn begin_label_edit(&mut self) {
        self.label_edit_mode_on = true;
        self.label_length = 0;
        self.label = ['\0';16];
    }

    pub fn confirm_label_edit(&mut self) {
        self.label_edit_mode_on = false;
        self.record_label()
    }

    pub fn cancel_label_edit(&mut self) {
        self.label_edit_mode_on = false;
    }

    pub fn apply_last_label(&mut self) {
        if self.label_length > 0 {
            self.record_label()
        }
    }
    pub fn palette_extract_on(&self) -> bool {
        self.palette_extract_on
    }

    pub fn capacity(&self) -> usize {
        self.navigator.capacity()
    }

    pub fn position(&self) -> Coords {
        self.navigator.position()
    }
    pub fn cells_per_row(&self) -> i32 {
        self.navigator.cells_per_row()
    }

    pub fn index_from_position(&self, coords: Coords) -> Option<usize> {
        self.navigator.index_from_position(coords)
    }
    pub fn entry_at_index(&self, index: usize) -> Option<&Entry> {
        if index < self.navigator.capacity() {
            Some(&self.entry_list[index])
        } else {
            None
        }
    }
    pub fn title_display(&self) -> String {
        if self.navigator.capacity() == 0 {
            return "".to_string()
        };
        let entry_title_display = &<Entry as Clone>::clone(&self.current_entry().unwrap()).title_display();
        format!("S:[{}] {} ordered by {} {}/{}  {} {} {} {}",
            self.max_selected,
            if self.select_start.is_some() { "…" } else { "" },
            if let Some(o) = self.order {
                o.to_string()
            } else {
                "??".to_string()
            },
            self.navigator.index(),
            self.navigator.capacity()-1,
            entry_title_display,
            if self.register.is_none() { String::from("") } else { format!("{}", self.register.unwrap()) },
            if self.real_size_on { "*" } else { "" },
            if self.label_edit_mode_on { format!("Label:{}", self.label.iter().collect::<String>()) } else { String::from("") }
            )
    }

    pub fn real_size(&self) -> bool {
        self.real_size_on
    }

    fn jump_to_name(&mut self, name: &String) {
        match self.entry_list.iter().position(|e| &e.original_file_path() == name) {
            Some(index) => { 
                self.navigator.move_to_index(index);
                self.real_size_on = false
            },
            None => {},
        }
    }

    pub fn can_move_abs(&self, coords: Coords) -> bool {
        self.navigator.can_move_abs(coords)
    }

    pub fn move_abs(&mut self, coords: Coords) {
        self.navigator.move_abs(coords)
    }

    pub fn can_move_rel(&self, direction: Direction) -> bool {
        self.navigator.can_move_rel(direction)
    }

    pub fn move_rel(&mut self, direction: Direction) {
        self.navigator.move_rel(direction)
    }

    pub fn move_forward_ten_pages(&mut self) {
        for _ in 0..10 {
            self.navigator.move_next_page()
        };
        println!("move forward ten pages")
    }

    pub fn move_backward_ten_pages(&mut self) {
        for _ in 0..10 { 
            self.navigator.move_prev_page()
        };
        println!("move backward ten pages")
    }

    pub fn move_to_register(&mut self) {
        if let Some(index) = self.register {
            self.navigator.move_to_index(index);
            self.register = None;
            self.real_size_on = false;
            println!("go to register index: {}", index)
        } else {
            self.register = Some(0);
            println!("start register index…")
        }
    }

    pub fn add_register_digit(&mut self, digit: usize ) {
        self.register = match self.register {
            Some(acc) => {
                let new_acc = acc * 10 + digit;
                if new_acc < self.navigator.capacity() { Some(new_acc) } else { Some(acc) }
            },
            None => {
                println!("no register index");
                None
            }
        }
    }

    pub fn delete_register_digit(&mut self) {
        self.register = self.register.map(|n| n / 10);
        if let Some(index) = self.register {
            println!("register index: {}", index)
        }
    }

    pub fn register_on(&self) -> bool {
        self.register.is_some()
    }

    pub fn sort_by(&mut self, order: Order) {
        println!("sort pictures by {}", order);
        if self.navigator.capacity() == 0 {
            return
        };
        let name = self.current_entry().unwrap().original_file_path();
        match order {
            Order::Label => self.entry_list.sort_by(|a, b| {
                let cmp = a.image_data.cmp_label(&b.image_data);
                if cmp == Equal {
                    a.original_file_path().cmp(&b.original_file_path())
                } else {
                    cmp
                }
            }),
            Order::Colors => self.entry_list.sort_by(|a, b| {
                let cmp = (a.image_data.colors).cmp(&b.image_data.colors);
                if cmp == Equal {
                    a.original_file_path().cmp(&b.original_file_path())
                } else {
                    cmp
                }
            }),
            Order::Palette => self.entry_list.sort_by(|a, b| { a.image_data.palette.cmp(&b.image_data.palette) }),
            Order::Date => self.entry_list.sort_by(|a, b| { a.modified_time.cmp(&b.modified_time) }),
            Order::Name => self.entry_list.sort_by(|a, b| { a.original_file_path().cmp(&b.original_file_path()) }),
            Order::Size => self.entry_list.sort_by(|a, b| { a.file_size.cmp(&b.file_size) }),
            Order::Value => self.entry_list.sort_by(|a,b| {
                let cmp = (a.image_data.rank as usize).cmp(&(b.image_data.rank as usize));
                if cmp == Equal {
                    a.original_file_path().cmp(&b.original_file_path())
                } else {
                    cmp
                }
            }),
            Order::Random => self.entry_list.shuffle(&mut thread_rng()),
        };
        self.order = Some(order);
        self.jump_to_name(&name)
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

    pub fn current_index(&self) -> usize {
        self.navigator.index()
    }
    pub fn toggle_real_size(&mut self) {
        if self.navigator.cells_per_row() == 1 {
            self.real_size_on = !self.real_size_on;
            println!("toggle real size")
        } else {
            println!("can't toggle real size in grid mode")
        }
    }

    pub fn can_move_to_index(&self, index: usize) -> bool {
        self.navigator.can_move_to_index(index)
    }

    pub fn move_to_index(&mut self, index: usize) {
        if self.navigator.can_move_to_index(index) {
            self.navigator.move_to_index(index);
            self.real_size_on = false;
            println!("move to picture #{}", index)
        } else {
            println!("can't move to picture #{}", index)
        }
    }

    pub fn move_to_random_index(&mut self) {
        self.navigator.move_to_random_index();
        self.real_size_on = false;
        println!("move to picture #{}", self.navigator.index())
    }

    pub fn move_next_page(&mut self) {
        self.navigator.move_next_page();
        self.real_size_on = false;
        println!("move to next page")
    }

    pub fn move_prev_page(&mut self) {
        self.navigator.move_prev_page();
        self.real_size_on = false;
        println!("move to prev page")
    }

    pub fn move_in_direction(&mut self, direction: Direction) {
        match direction {
            Direction::Right | Direction::Down => self.move_next_page(),
            Direction::Left | Direction::Up => self.move_prev_page(),
        }
    }

    pub fn page_changed(&self) -> bool {
        self.navigator.page_changed()
    }

    pub fn set_order_choice_on(&mut self) {
        self.order = None;
        println!("order choice on…");
    }

    pub fn quit(&self) {
        self.save_select_entries();
        println!("quit gallery show")
    }

    pub fn help(&self) {
        let content = "commands:\n\n\
        n: move next page\n\
        p: move prev page\n\
        j: move 10 pages forward\n\
        b: move 10 pages backward\n\
        z: move to first picture\n\
        r: move to a random picture\n\
        =: change order (followed by c,d,n,r,v for colors, date, name, random, value)\n\
        .: view picture (when in grid mode)\n\
        f: view real size (when not in grid mode)\n\
        ,: toggle selection\n\
        RET: start a selection/rank group\n\
        s: save selected entries\n\
        /: enter label edit mode\n\
        *: apply last label\n\
        ";
        println!("{}", &content)
    }
    pub fn copy_move_and_quit(&self) {
        self.save_select_entries();
        if let Some(target_path) = &self.copy_selection_target {
            println!("copy selection to target path");
            self.copy_select_entries(&target_path)
        };
        if let Some(target_path) = &self.move_selection_target {
            println!("move selection to target path");
            self.copy_select_entries(&target_path);
            self.delete_select_entries();
            delete_selection_file()
        };
        println!("quit gallery show")
    }

    pub fn order_choice_on(&self) -> bool {
        self.order.is_none()
    }

    pub fn toggle_select(&mut self) {
        assert!(self.entry_list.len() > 0);
        let index = self.navigator.index();
        let entry = &mut self.entry_list[index];
        entry.toggle_select();
        if entry.is_selected() { 
            self.max_selected += 1
        } else { 
           self.max_selected -= 1
        }; 
        if picture_io::save_image_data(&self.entry_list[index]).is_err() {
            println!("can't save image data {}", &self.entry_list[index].image_data_file_path())
        };
        self.navigator.refresh()

    }

    pub fn toggle_delete(&mut self) {
        assert!(self.entry_list.len() > 0);
        let index = self.navigator.index();
        let entry = &mut self.entry_list[index];
        entry.delete = ! entry.delete;
        self.navigator.refresh()
    }

    pub fn set_rank(&mut self, rank: Rank) {
        assert!(self.entry_list.len() > 0);
        let index = self.navigator.index();
        let entry = &mut self.entry_list[index];
        entry.set_rank(rank);
        if picture_io::save_image_data(&entry).is_err() {
            println!("can't save image data {}", &entry.image_data_file_path())
        };
        self.navigator.refresh()
    }

    pub fn record_label(&mut self) {
        assert!(self.entry_list.len() > 0);
        let index = self.navigator.index();
        let entry = &mut self.entry_list[index];
        entry.set_label(self.label_length, &self.label);
        println!("recording label {}", entry.image_data.label.iter().collect::<String>());
        if picture_io::save_image_data(&entry).is_err() {
            println!("can't save image data {}", &entry.image_data_file_path())
        };
        self.navigator.refresh()
    }

    pub fn point_remove_label(&mut self) {
        let index = self.navigator.index();
        self.label_length = 0;
        self.label = ['\0';16];
        match self.select_start {
            None => self.record_label(),
            Some(other) => {
                let (start,end) = if other <= index { (other,index) } else { (index,other) };
                println!("label: {}…{}", start, end);
                for i in start..end+1 {
                    let entry: &mut Entry = &mut self.entry_list[i];
                    entry.set_label(self.label_length, &self.label);
                    let _=  picture_io::save_image_data(entry);
                }
                self.select_start = None
            },
        };
        self.navigator.refresh()
    }

    pub fn select_page(&mut self, value: bool) {
        let start = self.navigator.start_cell_index();
        let end = min(start + self.navigator.max_cells() as usize, self.navigator.capacity());
        for i in start..end {
            let entry = &mut self.entry_list[i];
            entry.set_select(value);
            if picture_io::save_image_data(&self.entry_list[i]).is_err() {
                println!("can't save image data {}", &self.entry_list[i].image_data_file_path())
            };
            self.update_max_selected()
        };
        self.navigator.refresh()
    }

    pub fn select_all(&mut self, value: bool) {
        let start = 0;
        let end = self.navigator.capacity();
        for i in start..end {
            let entry = &mut self.entry_list[i];
            entry.set_select(value);
            if picture_io::save_image_data(&self.entry_list[i]).is_err() {
                println!("can't save image data {}", &self.entry_list[i].image_data_file_path())
            };
            self.update_max_selected()
        };
        self.navigator.refresh()
    }
    
    fn update_max_selected(&mut self) {
        self.max_selected = self.entry_list.iter().filter(|e| e.is_selected()).count()
    }

    pub fn point_label(&mut self) {
        if self.label_length > 0 {
            let index = self.navigator.index();
            match self.select_start {
                None => self.apply_last_label(),
                Some(other) => {
                    let (start,end) = if other <= index { (other,index) } else { (index,other) };
                    println!("label: {}…{}", start, end);
                    for i in start..end+1 {
                        let entry = &mut self.entry_list[i];
                        entry.set_label(self.label_length, &self.label);
                        let _=  picture_io::save_image_data(entry);
                    }
                    self.select_start = None
                },
            }
        };
        self.navigator.refresh()
    }

    pub fn select_point(&mut self) {
        let index = self.navigator.index();
        println!("select: {}…", index);
        self.select_start = Some(index)
    }

    pub fn point_select(&mut self) {
        let index = self.navigator.index();
        match self.select_start {
            None => {
                self.toggle_select();
                println!("picture #{} {}", index, if self.entry_list[index].image_data.selected { "selected" } else { "unselected" })
            },
            Some(other) => {
                let (start,end) = if other <= index { (other,index) } else { (index,other) };
                println!("select: {}…{}", start, end);
                for i in start..end+1 {
                    let entry = &mut self.entry_list[i];
                    entry.image_data.selected = true;
                    let _=  picture_io::save_image_data(entry);
                }
                self.select_start = None
            },
        };
        self.navigator.refresh();
        self.update_max_selected()
    }

    pub fn cancel_point(&mut self) {
        println!("point cancelled");
        self.select_start = None
    }

    pub fn point_rank(&mut self, rank: Rank) {
        let index = self.navigator.index();
        match self.select_start {
            None => {
                self.set_rank(rank);
                println!("picture #{} rank {}", index, rank)
            },
            Some(other) => {
                let (start,end) = if other <= index { (other,index) } else { (index,other) };
                println!("rank {}: {}…{}", rank, start, end);
                for i in start..end+1 {
                    self.entry_list[i].image_data.rank = rank
                }
                self.select_start = None
            }
        };
        self.navigator.refresh()
    }

    pub fn save_select_entries(&self) {
        let mut list: Vec<String> = Vec::new();
        let selection: Vec<&Entry> = self.entry_list.iter().filter(|e| e.image_data.selected).collect();
        for entry in selection.iter() {
            list.push(entry.original_file_path());
            list.push(entry.thumbnail_file_path());
            list.push(entry.image_data_file_path());
        };
        save_image_list(list);
    }

    pub fn copy_select_entries(&self, target: &str) {
        let target_path = Path::new(target);
        if !target_path.exists() {
            println!("directory doesn't exist: {}", target);
            return
        };
        let selection: Vec<&Entry> = self.entry_list.iter().filter(|e| e.image_data.selected).collect();
        for entry in selection {
            match copy_entry(entry, target_path) {
                Ok(_) => {},
                Err(e) => eprintln!("err copying entry {} to {}: {}", entry.original_file_path(), target_path.display(), e),
            }
        }
    }

    pub fn delete_select_entries(&self) {
        let selection: Vec<&Entry> = self.entry_list.iter().filter(|e| e.image_data.selected).collect();
        for entry in selection {
            delete_entry(entry)
        };
        delete_selection_file()
    }

    pub fn toggle_palette_extract(&mut self) {
        self.palette_extract_on = ! self.palette_extract_on;
        self.navigator.refresh()
    }

    pub fn delete_entries(&self) {
        let selection: Vec<&Entry> = self.entry_list.iter().filter(|e| e.delete).collect();
        for entry in selection {
            delete_entry(entry)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::cell::RefMut;
    use crate::make_entry;
    use std::time::SystemTime;
    use chrono::DateTime;

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
        repository.navigator.move_rel(Direction::Right);
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
            assert_eq!(true, entry.image_data.selected);
        } // reference is released here
        // second mutation occurs in that scope
        let mut repository: RefMut<'_, Repository> = repository_rc.borrow_mut();
        repository.toggle_select();
        let entry: &Entry = repository.current_entry().unwrap();
        assert_eq!(false, entry.image_data.selected);
    }

    #[test]
    fn after_two_select_points_a_group_of_entries_is_selected() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().navigator.move_rel(Direction::Down) }; // now current entry is #2 
        { repository_rc.borrow_mut().select_point() };
        { repository_rc.borrow_mut().navigator.move_rel(Direction::Up) }; // now current entry is #0
        { repository_rc.borrow_mut().point_select() }; // only entries 0,1,2 are selected
        let repository = repository_rc.borrow();
        for entry in &repository.entry_list[0..3] {
            assert_eq!(true, entry.image_data.selected)
        };
        assert_eq!(false, repository.entry_list[3].image_data.selected)
    }

    #[test]
    fn after_setting_rank_current_entries_has_a_new_rank() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().set_rank(Rank::ThreeStars) };
        let repository = repository_rc.borrow();
        assert_eq!(Rank::ThreeStars, repository.current_entry().unwrap().image_data.rank);
    }

    #[test]
    fn after_two_rank_points_a_group_on_entries_has_rank_changed() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().navigator.move_rel(Direction::Down) }; // now current entry is #2 
        { repository_rc.borrow_mut().select_point() };
        { repository_rc.borrow_mut().navigator.move_rel(Direction::Up) }; // now current entry is #0
        { repository_rc.borrow_mut().point_rank(Rank::TwoStars) }; // only entries 0,1,2 are ranked
        let repository = repository_rc.borrow();
        for entry in &repository.entry_list[0..3] {
            assert_eq!(Rank::TwoStars, entry.image_data.rank)
        };
        assert_eq!(Rank::OneStar, repository.entry_list[3].image_data.rank)
    }

    #[test]
    fn sorting_entries_by_date() {
        let repository_rc = Rc::new(RefCell::new(Repository::from_entries(example().clone(), 2)));
        { repository_rc.borrow_mut().sort_by(Order::Date) };
        { repository_rc.borrow_mut().navigator.move_to_index(0) };
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
        { repository_rc.borrow_mut().navigator.move_to_index(0) };
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
        { repository_rc.borrow_mut().navigator.move_to_index(0) };
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

