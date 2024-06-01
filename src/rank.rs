use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone,Copy,PartialEq,Debug)]
pub enum Rank {
   ThreeStars, TwoStars, OneStar, NoStar, 
}

impl Rank {

    pub fn show(&self) -> String {
        let limit = 3 - *self as usize;
        if limit > 0 {
            "☆".repeat(limit)
        } else {
            "".to_string()
        }
    }

}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Rank::ThreeStars => "☆☆☆",
            Rank::TwoStars => "☆☆",
            Rank::OneStar => "☆",
            Rank::NoStar => "_",
        })
    }
}
