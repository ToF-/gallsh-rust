use serde::{Deserialize, Serialize};
use core::cmp::Ordering;
use crate::rank::Rank;


#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct ImageData {
    pub colors: usize,
    pub rank: Rank,
    pub selected: bool,
    pub palette: [u32;9],
    pub label: String,
}

impl ImageData {

    pub fn new(colors: usize, rank: Rank) -> Self {
        ImageData {
            colors: colors,
            rank: rank,
            selected: false,
            palette: [0;9],
            label: String::new(),
        }
    }

    pub fn label(&self) -> Option<String> {
        if self.label.len() > 0 {
            Some(self.label.clone())
        } else {
            None
        }
    }

    pub fn cmp_label(&self, other: &ImageData) -> Ordering {
        match self.label() {
            Some(a) => match other.label() {
                Some(b) => a.cmp(&b),
                None => Ordering::Less,
            },
            None => match other.label() {
                Some(_) => Ordering::Greater,
                None => Ordering::Equal,
            }
        }
    }
}




