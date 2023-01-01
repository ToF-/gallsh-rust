use std::fs;
use std::path::PathBuf;
use std::env;
use gtk::prelude::*;
use gtk::gdk;
use gtk::glib;
use glib::clone;
use gtk::{Application, Button, Label, WindowPosition};
use gtk::gio;
use gio::SimpleAction;
use gio::SimpleActionGroup;

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

    let mut filename : Option<String> = None;

    let entries = fs::read_dir(gallshdir).unwrap();

    for entry in entries {
        let path : PathBuf = entry.unwrap().path();
        filename = Some(String::from(path.to_str().unwrap()));
        break;
    }

    let filename_label = Label::new(Some(&filename.unwrap()));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("gallsh")
        .default_width(1000)
        .default_height(1000)
        .child(&filename_label)
        .build();

    window.show_all();
}

