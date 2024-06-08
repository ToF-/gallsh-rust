#[derive(Clone)]
pub enum Direction {
    Left, Down, Right, Up,
}

impl From<(i32, i32)> for Direction {
    fn from(coords: (i32, i32)) -> Self {
        let (col,row) = coords;
        if col < 0 {
            Self::Left
        } else if col > 0 {
            Self::Right
        } else if row < 0 {
            Self::Up
        } else if row > 0 {
            Self::Down
        } else {
            Self::Right
        }
    }
}

impl Into<(i32, i32)> for Direction {
    fn into(self) -> (i32,i32) {
        match self {
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
            Self::Up =>    (0, -1),
            Self::Down =>  (0, 1),
        }
    }
}
