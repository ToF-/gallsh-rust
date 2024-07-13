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
    pub fn label(&self) -> Option<String> {
        if self.label.len() > 0 {
            Some(self.label.clone())
        } else {
            None
        }
    }

    pub fn cmp_label(&self, other: &ImageData) -> Ordering {
        if let Some(label_a) = self.label() {
            if let Some(label_b) = other.label() {
                label_a.cmp(&label_b)
            } else {
                Ordering::Less
            }
        } else {
            if other.label().is_some() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
    }
}

