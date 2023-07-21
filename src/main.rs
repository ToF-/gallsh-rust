use std::env;
use std::io;
use std::path::Path;
use std::io::Error;
use std::fs;
use std::path::PathBuf;
use glib::clone;
use gtk::prelude::*;
use gtk::{Application, gio, glib, Image };
use gio::SimpleAction;

fn get_files_in_directory(dir_path: &str) -> io::Result<Vec<String>> {
    // Get a list of all entries in the folder
    let entries = fs::read_dir(dir_path)?;

    // Extract the filenames from the directory entries and store them in a vector
    let file_names: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() {
                let full_path = Path::new(dir_path).join(path);
                full_path.to_str().map(|s| s.to_owned())
            } else {
                None
            }
        })
        .collect();

    Ok(file_names)
}

fn main() {
    let app = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    app.connect_activate(build_ui);
    app.set_accels_for_action("win.close", &["q"]);
    app.run();

    fn build_ui(app: &gtk::Application) {
        let mut gallshdir = String::from("images/");
        match env::var("GALLSHDIR")  {
            Ok(val) => gallshdir = String::from(val),
            Err(e) => {
                println!("GALLSHDIR: {e}\n default to \"{gallshdir}\"");
            }
        };

        let filenames = match get_files_in_directory(&gallshdir) {
            Err(msg) => panic!("{}", msg),
            Ok(result) => result,
        };
        //    let mut filenames = Vec::new();
        //    let entries = fs::read_dir(gallshdir)?
        //        .map(|result| result.map(|entry| entry.path()))
        //        .collect::<Result<Vec<_>, io::Error>>()?;
        //    dbg(entries);
        //    for entry in entries {
        //        let entry = entry?;
        //        let filename = entry?.file_name();
        //        filenames.push(filename.into_owned());
        //        break;
        //    }

        let ref filename = filenames[0];
        println!("reading image {}", filename);
        let image = Image::from_file(filename);

        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("gsr")
            .default_width(1000)
            .default_height(1000)
            .child(&image)
            .build();

        let action_close = SimpleAction::new("close", None);
        action_close.connect_activate(clone!(@weak window => move |_, _| {
            window.close();
        }));
        window.add_action(&action_close);
        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(|_, key, code, modifier_type| {
            println!("Key:{}\nCode:{}\nModifier type:{}", key, code, modifier_type);
            gtk::Inhibit(false)
        });
        window.add_controller(evk);

        window.present();
    }
}

