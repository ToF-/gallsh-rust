
type Coords = (i32, i32);

#[derive(Debug)]
struct Navigator {
    capacity: i32,
    cells_per_row: i32,
    max_cells: i32,
    start_cell_index: i32,
    position: Coords,
}

impl Navigator {
    pub fn new(capacity: i32, cells_per_row: i32) -> Self {
        Navigator {
            capacity: capacity,
            cells_per_row: cells_per_row,
            max_cells: cells_per_row * cells_per_row,
            start_cell_index: 0,
            position: (0,0),
        }
    }

    pub fn index_from_position(&self, position: Coords) -> Option<usize> {
        let result = (self.start_cell_index + position.0 + position.1 * self.cells_per_row) as usize;
        if result < self.capacity as usize {
            Some(result)
        } else {
            None
        }
    }

    pub fn index(&self) -> usize {
        self.index_from_position(self.position).unwrap() as usize
    }

    pub fn can_move_rel(&self, coords: Coords) -> bool {
        let position = (self.position.0 + coords.0, self.position.1 + coords.1);
        position.0 >= 0
            && position.0 < self.cells_per_row
            && position.1 >= 0
            && position.1 < self.cells_per_row
            && !self.index_from_position(position).is_none()

    }

    pub fn move_rel(&mut self, coords: Coords) {
        if self.can_move_rel(coords) {
            self.position = (self.position.0 + coords.0, self.position.1 + coords.1)
        } else {
            panic!("Navigator {:?} can't move to this relative position: {:?}", self, coords);
        }
    }

    pub fn can_move_abs(&self, index: usize) -> bool {
        index < self.capacity as usize
    }

    pub fn move_abs(&mut self, n: usize) {
        if self.can_move_abs(n) {
            let index = n as i32;
            let rel_index = index % self.max_cells;
            self.start_cell_index = index - rel_index;
            self.position = (rel_index % self.cells_per_row, rel_index / self.cells_per_row);
        } else {
            panic!("Navigator {:?} can't move to this absolute position: {:?}", self, n);
        }
    }

    pub fn move_next_page(&mut self) {
        self.position = (0,0);
        if self.start_cell_index + self.max_cells < self.capacity {
            self.start_cell_index += self.max_cells
        } else {
            self.start_cell_index = 0
        }
    }

    pub fn move_prev_page(&mut self) {
        self.position = (0,0);
        if self.start_cell_index > 0 {
            self.start_cell_index -= self.max_cells
        } else {
            self.start_cell_index = self.capacity - self.capacity % self.max_cells
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_index_before_any_move_is_given_index() {
        let navigator = Navigator::new(10, 4);
        assert_eq!(0 as usize, navigator.index());
    }

    #[test]
    fn after_a_move_index_has_changed() {
        let mut navigator = Navigator::new(10, 4);
        navigator.move_rel((1, 0));
        assert_eq!(1, navigator.index());
        navigator.move_rel((0,1));
        assert_eq!(5, navigator.index());
        navigator.move_rel((-1,0));
        assert_eq!(4, navigator.index());
        navigator.move_rel((0,-1));
        assert_eq!(0, navigator.index());
    }

    #[test]
    fn after_next_page_or_prev_page_index_is_changed_and_aligned_to_a_page() {
        let mut navigator = Navigator::new(100, 4);
        assert_eq!(0, navigator.index());
        navigator.move_rel((0,1));
        navigator.move_rel((0,1));
        navigator.move_next_page();
        assert_eq!(16, navigator.index());
        navigator.move_rel((0,1));
        navigator.move_rel((0,1));
        assert_eq!(16+4+4, navigator.index());
        navigator.move_prev_page();
        assert_eq!(0, navigator.index());
    }

    #[test]
    fn after_prev_page_on_first_page_index_is_start_of_last_page() {
        let mut navigator = Navigator::new(10,2);
        navigator.move_prev_page();
        assert_eq!(8, navigator.index());
        navigator = Navigator::new(10,3);
        navigator.move_prev_page();
        assert_eq!(9, navigator.index());
    }

    #[test]
    fn after_next_page_on_last_page_index_is_start_of_first_page() {
        let mut navigator = Navigator::new(10,2);
        navigator.move_next_page();
        navigator.move_next_page();
        navigator.move_next_page();
        assert_eq!(0, navigator.index());
    }

    #[test]
    fn relative_move_can_be_checked() {
        let mut navigator = Navigator::new(10, 2);
        assert_eq!(true, navigator.can_move_rel((1,0)));
        assert_eq!(false, navigator.can_move_rel((-1,0)));
        assert_eq!(false, navigator.can_move_rel((2,0)));
        assert_eq!(false, navigator.can_move_rel((0,-1)));
        assert_eq!(false, navigator.can_move_rel((0,2)));
        navigator.move_rel((1,0));
        assert_eq!(true, navigator.can_move_rel((-1,0)));
    }

    #[test]
    fn on_last_page_move_is_checked_against_capacity() {
        let mut navigator = Navigator::new(10, 2);
        navigator.move_next_page();
        assert_eq!(true, navigator.can_move_rel((0,1)));
        navigator.move_next_page();
        assert_eq!(false, navigator.can_move_rel((0,1))); // because that would move to index 10
    }

    #[test]
    fn absolute_move_can_be_checked() {
        let navigator = Navigator::new(10, 2);
        assert_eq!(true, navigator.can_move_abs(3));
    }

    #[test]
    fn after_absolute_move_page_can_be_changed() {
        let mut navigator = Navigator::new(10,2);
        navigator.move_abs(7);
        assert_eq!(4, navigator.start_cell_index);
        assert_eq!((1,1), navigator.position);
        assert_eq!(7, navigator.index());
    }

    #[test]
    #[should_panic]
    fn navigator_should_panic_if_move_abs_where_not_allowed() {
        let mut navigator = Navigator::new(10, 2);
        navigator.move_abs(4807);
    }
    #[test]
    #[should_panic]
    fn navigator_should_panic_if_move_rel_where_not_allowed() {
        let mut navigator = Navigator::new(10, 2);
        navigator.move_rel((-1,0));
    }

    #[test]
    fn index_from_position_can_yield_a_possible_index() {
        let mut navigator = Navigator::new(10, 2);
        assert_eq!(Some(3), navigator.index_from_position((1,1)));
        navigator.move_next_page();
        assert_eq!(Some(7), navigator.index_from_position((1,1)));
        navigator.move_next_page();
        assert_eq!(None, navigator.index_from_position((1,1))); // because that would be 11
    }
}
