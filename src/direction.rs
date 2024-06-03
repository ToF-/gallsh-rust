use crate::navigator::Coords;

pub enum Direction {
    Left, Down, Right, Up,
}

impl Direction {
    pub fn into_coords(&self) -> Coords {
        match self {
            Self::Left => (-1, 0),
            Self::Down => (0, 1),
            Self::Right => (1, 0),
            Self::Up => (0, -1),
        }
    }
}

