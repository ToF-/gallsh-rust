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

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct FormerImageData {
    pub colors: usize,
    pub rank: Rank,
    pub selected: bool,
    pub palette: [u32;9],
    pub label_length: usize,
    pub label: [char;16],
}
impl FormerImageData {
    pub fn label(&self) -> Option<String> {
        if self.label_length > 0 {
            Some(self.label.iter().take_while(|c| **c != '\0').collect::<String>())
        } else {
            None
        }
    }
}

pub fn image_data_from_former_image_data(former: &FormerImageData) -> ImageData {
   ImageData {
       colors: former.colors,
       rank: former.rank,
       selected: former.selected,
       palette: former.palette,
       label: match former.label() {
           Some(s) => s,
           None => String::new(),
       }
   }
}

