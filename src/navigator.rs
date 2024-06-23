use crate::Direction;
use rand::Rng;
use rand::thread_rng;

pub type Coords = (i32, i32);

#[derive(Clone,Debug)]
pub struct Navigator {
    capacity: i32,
    cells_per_row: i32,
    max_cells: i32,
    start_cell_index: i32,
    position: Coords,
    page_changed: bool,
}

impl Navigator {
    pub fn new(capacity: i32, cells_per_row: i32) -> Self {
        Navigator {
            capacity: capacity,
            cells_per_row: cells_per_row,
            max_cells: cells_per_row * cells_per_row,
            start_cell_index: 0,
            position: (0,0),
            page_changed: true,
        }
    }

    pub fn start_cell_index(&self) -> usize {
        self.start_cell_index as usize
    }
    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }

    pub fn cells_per_row(&self) -> i32 {
        self.cells_per_row
    }

    pub fn max_cells(&self) -> i32 {
        self.max_cells
    }

    pub fn position(&self) -> Coords {
        self.position
    }

    pub fn page_changed(&self) -> bool {
        self.page_changed
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
        match self.index_from_position(self.position) {
            Some(n) => n,
            None => {
                println!("unexpected position: {:?}", self.position);
                0
            },
        }
    }

    pub fn can_move_rel(&self, direction: Direction) -> bool {
        let coords: Coords = direction.into();
        let position = (self.position.0 + coords.0, self.position.1 + coords.1);
        position.0 >= 0
            && position.0 < self.cells_per_row
            && position.1 >= 0
            && position.1 < self.cells_per_row
            && self.index_from_position(position).is_some()

    }

    pub fn move_rel(&mut self, direction: Direction) {
        let (col, row) = direction.into();
        self.position = (self.position.0 + col, self.position.1 + row);
        self.page_changed = false;
        assert!(self.position.0 >= 0 && self.position.0 < self.cells_per_row && self.position.1 >= 0 && self.position.1 < self.cells_per_row)
    }

    pub fn can_move_abs(&self, position: Coords) -> bool {
        position.0 >= 0
            && position.0 < self.cells_per_row
            && position.1 >= 0
            && position.1 < self.cells_per_row
            && self.index_from_position(position).is_some()
    }

    pub fn move_abs(&mut self, position: Coords) {
        if self.can_move_abs(position) {
            self.position = position;
            self.page_changed = false;

        } else {
            panic!("Navigator {:?} can't move to this position: {:?}", self, position)
        }
    }

    pub fn can_move_to_index(&self, index: usize) -> bool {
        index < self.capacity as usize
    }

    pub fn move_to_index(&mut self, n: usize) {
        if self.can_move_to_index(n) {
            let index = n as i32;
            let rel_index = index % self.max_cells;
            self.start_cell_index = index - rel_index;
            self.position = (rel_index % self.cells_per_row, rel_index / self.cells_per_row);
            self.page_changed = true;
        } else {
            panic!("Navigator {:?} can't move to this absolute position: {:?}", self, n);
        }
    }

    pub fn move_to_random_index(&mut self) {
        let index = thread_rng().gen_range(0..self.capacity());
        self.move_to_index(index);
    }

    pub fn move_next_page(&mut self) {
        self.position = (0,0);
        if self.start_cell_index + self.max_cells < self.capacity {
            self.start_cell_index += self.max_cells
        } else {
            self.start_cell_index = 0
        };
        self.page_changed = true;
    }

    pub fn move_prev_page(&mut self) {
        self.position = (0,0);
        if self.start_cell_index > 0 {
            self.start_cell_index -= self.max_cells
        } else {
            self.start_cell_index = ((self.capacity-1) / self.max_cells) * self.max_cells
        };
        self.page_changed = true;
    }

    pub fn refresh(&mut self) {
        self.page_changed = true
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
    fn after_a_move_rel_index_has_changed() {
        let mut navigator = Navigator::new(10, 4);
        navigator.move_rel(Direction::Right);
        assert_eq!(1, navigator.index());
        navigator.move_rel(Direction::Down);
        assert_eq!(5, navigator.index());
        navigator.move_rel(Direction::Left);
        assert_eq!(4, navigator.index());
        navigator.move_rel(Direction::Up);
        assert_eq!(0, navigator.index());
    }
    #[test]
    fn after_a_move_abs_index_has_changed() {
        let mut navigator = Navigator::new(100, 4);
        navigator.move_abs((3, 2));
        assert_eq!(11, navigator.index()); 
        navigator.move_abs((2,3));
        assert_eq!(14, navigator.index());
    }

    #[test]
    fn after_next_page_or_prev_page_index_is_changed_and_aligned_to_a_page() {
        let mut navigator = Navigator::new(100, 4);
        assert_eq!(0, navigator.index());
        navigator.move_rel(Direction::Right);
        navigator.move_rel(Direction::Right);
        navigator.move_next_page();
        assert_eq!(16, navigator.index());
        navigator.move_rel(Direction::Down);
        navigator.move_rel(Direction::Down);
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
        assert_eq!(true, navigator.can_move_rel(Direction::Right));
        assert_eq!(false, navigator.can_move_rel(Direction::Left));
        assert_eq!(false, navigator.can_move_rel(Direction::Up));
        navigator.move_rel(Direction::Right);
        assert_eq!(true, navigator.can_move_rel(Direction::Left));
    }

    #[test]
    fn on_last_page_move_is_checked_against_capacity() {
        let mut navigator = Navigator::new(10, 2);
        navigator.move_next_page();
        assert_eq!(true, navigator.can_move_rel(Direction::Right));
        navigator.move_next_page();
        assert_eq!(false, navigator.can_move_rel(Direction::Down)); // because that would move to index 10
    }

    #[test]
    fn move_to_index_can_be_checked() {
        let navigator = Navigator::new(10, 2);
        assert_eq!(true, navigator.can_move_to_index(3));
    }

    #[test]
    fn after_move_to_index_page_can_be_changed() {
        let mut navigator = Navigator::new(10,2);
        navigator.move_to_index(7);
        assert_eq!(4, navigator.start_cell_index);
        assert_eq!((1,1), navigator.position);
        assert_eq!(7, navigator.index());
    }

    #[test]
    #[should_panic]
    fn navigator_should_panic_if_move_to_index_where_not_allowed() {
        let mut navigator = Navigator::new(10, 2);
        navigator.move_to_index(4807);
    }
    #[test]
    #[should_panic]
    fn navigator_should_panic_if_move_rel_where_not_allowed() {
        let mut navigator = Navigator::new(10, 2);
        navigator.move_rel(Direction::Left);
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
