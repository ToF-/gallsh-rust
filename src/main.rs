use std::fs;
use std::path::PathBuf;
use std::env;
use gtk::prelude::*;
use gtk::glib;
use glib::clone;
use gtk::{Application, Image, Label};

fn main() {
    let app = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &gtk::Application) {
    let mut gallshdir = String::from("images/");
    match env::var("GALLSHDIR")  {
        Ok(val) => gallshdir = String::from(val),
        Err(e) => {
            println!("GALLSHDIR: {e}\n default to \"{gallshdir}\"");
        }
    };
    println!("GALLSHDIR={gallshdir}");

    let mut filename : String = "".to_string();

    let entries = fs::read_dir(gallshdir).unwrap();

    for entry in entries {
        let path : PathBuf = entry.unwrap().path();
        filename = String::from(path.to_str().unwrap());
        break;
    }

    let filename_label = Label::new(Some(&filename));

    let image = Image::from_file(&filename);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("gallsh")
        .default_width(1000)
        .default_height(1000)
        .child(&filename_label)
        .child(&image)
        .build();

    window.show_all();
}

