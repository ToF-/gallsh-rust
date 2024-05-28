use gtk::gio::File;
use std::io::{Result,Error, ErrorKind};

pub fn thumbnail_file(file_path: &str) -> Result<File> {
    Err(Error::new(ErrorKind::Other, "foo"))
}

pub fn original_file(file_path: &str) -> Result<File> {
    Err(Error::new(ErrorKind::Other, "foo"))
}
