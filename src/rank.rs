use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone,Copy,PartialEq,Debug)]
pub enum Rank {
   ThreeStars, TwoStars, OneStar, NoStar, 
}

impl Rank {
    pub fn from_usize(v: usize) -> Self {
        match v {
            0 => Rank::ThreeStars,
            1 => Rank::TwoStars,
            2 => Rank::OneStar,
            _ => Rank::NoStar,
        }
    }

    pub fn show(&self) -> String {
        let limit = 3 - *self as usize;
        if limit > 0 {
            "☆".repeat(limit)
        } else {
            "".to_string()
        }
    }

}
