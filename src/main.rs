use std::env;
use std::fs;
use std::path::PathBuf;
use gtk4 as gtk;
use glib::clone;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, gio, Image};
use gio::SimpleAction;

fn main() {
    let app = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    app.connect_activate(build_ui);
    app.set_accels_for_action("win.close", &["q"]);
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
    let mut filename : String = "".to_string();
    let entries = fs::read_dir(gallshdir).unwrap();
    for entry in entries {
        let path : PathBuf = entry.unwrap().path();
        filename = String::from(path.to_str().unwrap());
        break;
    }

    let image = Image::from_file(&filename);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(&filename)
        .default_width(1000)
        .default_height(1000)
        .child(&image)
        .build();

    let action_close = SimpleAction::new("close", None);
    action_close.connect_activate(clone!(@weak window => move |_, _| {
        window.close();
    }));
    window.add_action(&action_close);

    window.present();
}

