use clap::Parser;
use core::time::{Duration};
use gio::SimpleAction;
use glib::clone;
use glib::timeout_add_local;
use gtk::prelude::*;
use gtk::{self, Application, gdk, gio, glib, Image};
use rand::{thread_rng, Rng};
use std::cell::Cell;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::rc::Rc;

// a struct to keep track of navigating in image files
#[derive(Clone, Copy, Debug)]
struct Index {
    selected: usize,
    maximum:  usize,
}

impl Index {
    fn new(capacity: usize) -> Self {
        Index {
            selected: 0,
            maximum: capacity - 1,
        }
    }

    fn next(&mut self) {
        self.selected = if self.selected < self.maximum { self.selected + 1 } else { 0 } ;

    }
    fn prev(&mut self) {
        self.selected = if self.selected > 0 { self.selected - 1 } else { self.maximum };
    }

    fn random(&mut self) {
        self.selected = thread_rng().gen_range(0..self.maximum + 1);
    }
}

enum Navigate {
    Current,
    Next,
    Prev,
    Random,
}

fn get_files_in_directory(dir_path: &str, pattern: &Option<String>) -> io::Result<Vec<String>> {
    let entries = fs::read_dir(dir_path)?;
    let file_names: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let valid_ext = if let Some(ext) = path.extension() {
                ext == "jpg" || ext == "jpeg" || ext == "png"
            } else {
                false
            };
            let p = if let Some(s) = pattern {
                path.is_file() && path.to_str().unwrap().contains(s)
            } else {
                path.is_file()
            };
            if valid_ext && p {
                let full_path = Path::new(dir_path).join(path);
                full_path.to_str().map(|s| s.to_owned())
            } else {
                None
            }
        })
        .collect();
    Ok(file_names)
}

// declarative setting of arguments
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

    /// Ordered display (or random)
    #[arg(short, long, default_value_t = false)]
    ordered: bool,

    /// Timer delay for next picture
    #[arg(short, long)]
    timer: Option<u64>,
}

const DEFAULT_DIR :&str  = "images/";
const ENV_VARIABLE :&str = "GALLSHDIR";

fn main() {
    let args = Args::parse();
    let gallshdir = env::var(ENV_VARIABLE);
    let path = if let Ok(s) = gallshdir {
        String::from(s)
    } else {
        println!("GALLSHDIR variable not set. Using {} as default.", DEFAULT_DIR);
        String::from(DEFAULT_DIR)
    };
    println!("searching images in {}", path);

    // build an application with some css characteristics
    let application = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    application.connect_startup(|_| {
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data("window { background-color:black;} image { margin:10em ; }");
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css_provider,
            1000,
        );
    });

    let pattern = args.pattern;
    // clone! passes a strong reference to pattern in the closure that activates the application
    application.connect_activate(clone!(@strong pattern => move |application: &gtk::Application| { 

        // get all the filenames in the directory that match pattern (or all if None)
        let mut filenames = match get_files_in_directory(&path, &pattern) {
            Err(msg) => panic!("{}", msg),
            Ok(result) => result,
        };
        filenames.sort();
        println!("{} files selected", filenames.len());

        // build the main window
        let image = Image::new();
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .default_width(1000)
            .default_height(1000)
            .child(&image)
            .build();

        let mut index = Index::new(filenames.len());
        if !args.ordered {
            index.random()
        };
        let index_rc = Rc::new(Cell::new(index));


        // add an action to close the window
        let action_close = SimpleAction::new("close", None);
        action_close.connect_activate(clone!(@strong index_rc, @weak window => move |_, _| {
            window.close();
        }));
        window.add_action(&action_close);

        // add an action to show random image
        let action = SimpleAction::new("random", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            show_image(&filenames, &image, &index_rc, &window, Navigate::Random);
        }));
        window.add_action(&action);

        // add an action to show next image
        let action = SimpleAction::new("next", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            show_image(&filenames, &image, &index_rc, &window, Navigate::Next);
        }));
        window.add_action(&action);
        
        // add an action to show prev image
        let action = SimpleAction::new("prev", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            show_image(&filenames, &image, &index_rc, &window, Navigate::Prev);
        }));
        window.add_action(&action);

        // add an action to show next or random image
        let action = SimpleAction::new("randnext", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            if args.ordered {
                show_image(&filenames, &image, &index_rc, &window, Navigate::Next);
            } else {
                show_image(&filenames, &image, &index_rc, &window, Navigate::Random);
            }
        }));
        window.add_action(&action);
        let evk = gtk::EventControllerKey::new();
        // handle space key event
        evk.connect_key_pressed(clone!(@strong filenames, @strong image, @strong index_rc, @strong window => move |_, key, _, _| {
            if let Some(s) = key.name() {
                match s.as_str() {
                    "space" => if args.ordered { 
                        show_image(&filenames, &image, &index_rc, &window, Navigate::Next)
                    } else {
                        show_image(&filenames, &image, &index_rc, &window, Navigate::Random)
                    }, 
                        _ => { },
                }
            } else { 
            } ;
            gtk::Inhibit(false)
        }));
        window.add_controller(evk);
        // show the first file
        show_image(&filenames, &image, &index_rc, &window, Navigate::Current);

        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong filenames, @strong image, @strong index_rc, @strong window => move | | { 
                if args.ordered { 
                    show_image(&filenames, &image, &index_rc, &window, Navigate::Next)
                } else {
                    show_image(&filenames, &image, &index_rc, &window, Navigate::Random)
                };
                Continue(true) 
            }));
    };
        window.present();
    }));
    application.set_accels_for_action("win.close", &["q"]);
    application.set_accels_for_action("win.random", &["r"]);
    application.set_accels_for_action("win.next", &["n"]);
    application.set_accels_for_action("win.prev", &["p"]);
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn show_image(filenames: &Vec<String>, image: &Image, index_rc:&Rc<Cell<Index>>, window: &gtk::ApplicationWindow, navigate:Navigate) {
    let mut index = index_rc.get();
    let filename = &filenames[index.selected];
    match navigate {
        Navigate::Next => index.next(),
        Navigate::Prev => index.prev(),
        Navigate::Random => index.random(),
        Navigate::Current => { } ,
    }
    index_rc.set(index);
    image.set_from_file(Some(filename.clone()));
    window.set_title(Some(filename.as_str()));
}
