use clap::Parser;
use gio::SimpleAction;
use glib::clone;
use gtk::prelude::*;
use gtk::{self, Application, gdk, gio, glib, Image};
use rand::{thread_rng, Rng};
use std::cell::Cell;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::rc::Rc;

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

/// Gallery Show
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Pattern that displayed files must have
    #[arg(short, long)]
    pattern: Option<String>,

    /// Maximized window
    #[arg(short, long, default_value_t = false)]
    maximized: bool,
}

fn main() {
    let args = Args::parse();

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

    let maximized = args.maximized;
    app.connect_activate(clone!(@strong maximized => move |app: &gtk::Application| { 
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

        if maximized { window.fullscreen() };
        window.present();
    }));
    app.set_accels_for_action("win.close", &["q"]);
    let empty: Vec<String> = vec![];
    app.run_with_args(&empty);

}

