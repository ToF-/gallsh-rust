use crate::Rank;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct ImageData {
    pub colors: usize,
    pub rank: Rank,
    pub selected: bool,
    pub palette: [u32;9],
    pub label_length: usize,
    pub label: [char;16],
}

impl ImageData {
    pub fn label(&self) -> Option<String> {
        if self.label_length > 0 {
            Some(self.label.iter().collect::<String>())
        } else {
            None
        }
    }
}
