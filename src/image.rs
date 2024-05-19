use image;
use image::{Rgba, ImageError, ImageResult, open};
use std::io::{Error, ErrorKind};
use image::GenericImageView;
use std::collections::HashSet;



fn rgba_key(rgba: Rgba<u8>) -> u32 {
    let mut result: u32 = 0;
    for i in 0..4 {
        result <<= 8;
        result |= rgba[i] as u32
    };
    result 
}
pub fn get_image_color_size(file_path: &str) -> ImageResult<usize> {
    println!("getting color size of {}", file_path);
    match open(file_path) {
        Ok(dynamic_image) => {
            let iter: Vec<_>= dynamic_image.pixels().collect();
            let mut colors: HashSet<u32> = HashSet::new();
            for i in iter {
                let rgba = i.2;
                colors.insert(rgba_key(rgba));
            };
            Ok(colors.len())
        },
        Err(err) => {
            println!("error getting image {} : {}", file_path, err);
            Err(err)
        },
    }
}
