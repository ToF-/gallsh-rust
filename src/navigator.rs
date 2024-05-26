
type Coords = (i32, i32);

struct Navigator {
    cells_per_row: i32,
    start_cell_index: i32,
    position: Coords,
}

impl Navigator {
    pub fn new(cells_per_row: i32) -> Self {
        Navigator {
            cells_per_row: cells_per_row,
            start_cell_index: 0,
            position: (0,0),
        }
    }

    pub fn index(&self) -> usize {
        (self.start_cell_index + self.position.0 + self.position.1 * self.cells_per_row) as usize
    }

    pub fn move_rel(&mut self, coords: Coords) {
        self.position = (self.position.0 + coords.0, self.position.1 + coords.1)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_index_before_any_move_is_given_index() {
        let navigator = Navigator::new(4);
        assert_eq!(0 as usize, navigator.index());
    }

    #[test]
    fn after_a_move_righ_index_has_changed() {
        let mut navigator = Navigator::new(4);
        navigator.move_rel((1, 0));
        assert_eq!(1, navigator.index());
    }
}
