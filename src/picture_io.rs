use crate::Rank;
use crate::image::get_image_color;
use std::fs::read_to_string;
use crate::Entry;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use thumbnailer::error::ThumbResult;
use thumbnailer::create_thumbnails;
use thumbnailer::ThumbnailSize;
use std::ffi::OsStr;






use std::io::{Result,Error, ErrorKind};

pub fn thumbnail_file(file_path: &str) -> Result<File> {
    Err(Error::new(ErrorKind::Other, "foo"))
}

pub fn original_file(file_path: &str) -> Result<File> {
    Err(Error::new(ErrorKind::Other, "foo"))
}


pub fn set_original_picture_file(picture: &gtk::Picture, entry: &Entry) -> Result<()> {
    let original = entry.original_file_path();
    let path = Path::new(&original);
    if path.exists() {
        picture.set_filename(Some(original));
        Ok(())
    } else {
        Err(Error::new(ErrorKind::Other, format!("file {} doesn't exist", original)))
    }
}

fn write_thumbnail<R: std::io::Seek + std::io::Read>(reader: BufReader<R>, extension: &str, mut output_file: File) -> ThumbResult<()> {
    let mime = match extension {
        "jpg" | "jpeg" | "JPG" | "JPEG" => mime::IMAGE_JPEG,
        "png" | "PNG" => mime::IMAGE_PNG,
        _ => panic!("wrong extension"),
    };
    let mut thumbnails = match create_thumbnails(reader, mime, [ThumbnailSize::Small]) {
        Ok(tns) => tns,
        Err(err) => {
            println!("error while creating thumbnails:{:?}", err);
            return Err(err)
        },
    };
    let thumbnail = thumbnails.pop().unwrap();
    let write_result = match extension {
        "jpg" | "jpeg" | "JPG" | "JPEG" => thumbnail.write_jpeg(&mut output_file,255),
        "png" | "PNG" => thumbnail.write_png(&mut output_file),
        _ => panic!("wrong extension"),
    };
    match write_result {
        Err(err) => {
            println!("error while writing ihunbnail:{}", err);
            Err(err)
        },
        ok => ok,
    }
}

fn create_thumbnail(entry: &Entry) -> Result<()> {
    let original = entry.original_file_path();
    let thumbnail = entry.thumbnail_file_path();
    println!("creating thumbnail {}", thumbnail);
    match File::open(original.clone()) {
        Err(err) => Err(err),
        Ok(input_file) => {
            let source_path = Path::new(&original);
            let extension = match source_path.extension()
                .and_then(OsStr::to_str) { 
                    None => return Err(Error::new(ErrorKind::Other, format!("source file has no extension"))),
                    Some(ext) => ext,
                };
            let reader = BufReader::new(input_file);
            let output_file = match File::create(thumbnail) {
                Err(err) => return Err(err),
                Ok(file) => file,
            };
            match write_thumbnail(reader, extension, output_file) {
                Err(err) => Err(Error::new(ErrorKind::Other, err)),
                Ok(_) => Ok (()),
            }
        },
    }
}

pub fn set_thumbnail_picture_file(picture: &gtk::Picture, entry: &Entry) -> Result<()> {
    let thumbnail = entry.thumbnail_file_path();
    let path = Path::new(&thumbnail);
    if path.exists() {
        picture.set_filename(Some(thumbnail));
        Ok(())
    } else {
        match create_thumbnail(entry) {
            Ok(()) => {
                picture.set_filename(Some(thumbnail));
                Ok(())
            },
            err => err,
        }
    }
}

pub fn set_image_data(mut entry: &mut Entry) -> Result<()> {
    let image_data = entry.image_data_file_path();
    let path = Path::new(&image_data);
    if path.exists() {
        match read_to_string(path.clone()) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok((colors, rank)) => {
                    entry.colors = colors;
                    entry.rank = rank;
                    Ok(())
                },
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err),
        }
    } else {
        match get_image_color(&entry.original_file_path()) {
            Ok(colors) => {
                match File::create(path.clone()) {
                    Ok(file) => {
                        let data = (colors, Rank::NoStar);
                        match serde_json::to_writer(file, &data) {
                            Ok(_) => Ok(()),
                            Err(err) => Err(err.into()),
                        }
                    },
                    Err(err) => Err(err),
                }
            },
            Err(err) => Err(Error::new(ErrorKind::Other,err)),
        }
    }
}
