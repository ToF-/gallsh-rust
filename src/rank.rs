use serde::{Deserialize, Serialize};

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
