use rand::{thread_rng, Rng};
use std::cell::Cell;
use std::rc::Rc;
use std::env;
use std::io;
use std::path::Path;
use std::fs;
use glib::clone;
use gtk::prelude::*;
use gtk::{Application, gdk, gio, glib, Image};
use gio::SimpleAction;

fn get_files_in_directory(dir_path: &str) -> io::Result<Vec<String>> {
    let entries = fs::read_dir(dir_path)?;
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

    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data("window { background-color:black;} image { margin:10em ; }");
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &provider,
            1000,
        );
    });

    app.connect_activate(build_ui);
    app.set_accels_for_action("win.close", &["q"]);
    app.run();

    fn build_ui(app: &gtk::Application) {
        let selected_image_index:Rc<Cell<usize>> = Rc::new(Cell::new(0));
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
        let image = Image::new();
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("gsr")
            .default_width(1000)
            .default_height(1000)
            .child(&image)
            .build();

        let mut rng = thread_rng();
        selected_image_index.set(rng.gen_range(0..filenames.len()));

        let index = selected_image_index.get();
        image.set_from_file(Some(&filenames[index]));

        let action_close = SimpleAction::new("close", None);
        action_close.connect_activate(clone!(@weak window => move |_, _| {
            window.close();
        }));
        window.add_action(&action_close);
        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(move |_, key, _, _| {
            if let Some(s) = key.name() {
                let current = selected_image_index.get();
                match s.as_str() {
                    "n" => {
                        selected_image_index.set(if current == filenames.len()-1 { 0 } else { current + 1 });
                    },
                    "p" => {
                        selected_image_index.set(if current == 0 { filenames.len()-1 } else { current - 1});
                    },
                    "r" => {
                        let mut rng = thread_rng();
                        selected_image_index.set(rng.gen_range(0..filenames.len()));
                    },
                    _ => {
                    },
                };
                let index = selected_image_index.get();
                image.set_from_file(Some(&filenames[index]));
            };
            gtk::Inhibit(false)
        });
        window.add_controller(evk);

        window.present();
    }
}

