use gio::SimpleAction;
use glib::clone;
use glib::timeout_add_local;
use gtk::prelude::*;
use gtk::{self, Application, gdk, gio, glib, Image};
use rand::{thread_rng, Rng};
use std::cell::Cell;
use std::env;
use std::io;
use std::fs::OpenOptions;
use std::fs::read_to_string;
use std::path::Path;
use std::io::{Write};
use std::rc::Rc;
use std::time::{Duration};
use clap::Parser;
use walkdir::WalkDir;


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
    fn set(&mut self, value: usize) {
        if value >= 0 && value < self.maximum {
            self.selected = value;
        } else {
            println!("index {} out of range, set to 0", value);
            self.selected = 0;
        }
    }
}

enum Navigate {
    Current,
    Next,
    Prev,
    Random,
}

fn get_files_from_reading_list(reading_list: &String) -> io::Result<Vec<String>> {
    match read_to_string(reading_list) {
        Ok(content) => Ok(content.lines().map(String::from).collect()),
        Err(msg) => Err(msg)
    }
}
fn get_files_in_directory(dir_path: &str, opt_pattern: &Option<String>) -> io::Result<Vec<String>> {
    let mut file_names: Vec<String> = Vec::new();
    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.into_path();
        let valid_ext = if let Some(ext) = path.extension() {
            ext == "jpg" || ext == "jpeg" || ext == "png"
        } else {
            false
        };
        let pattern_present = if let Some(pattern) = opt_pattern {
            path.is_file() && path.to_str().map(|filename| filename.contains(pattern)) == Some(true)
        } else {
            path.is_file()
        };
        if valid_ext && pattern_present {
            if let Some(full_name) = path.to_str() {
                file_names.push(full_name.to_string().to_owned());
            }
        }
    };
    Ok(file_names)
}

// declarative setting of arguments
/// Gallery Show
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Pattern that displayed files must have
struct Args {
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

    /// Directory to search
    #[arg(short, long)]
    directory: Option<String>,

    /// Selection File
    #[arg(short, long)]
    selection: Option<String>,

    /// Reading List
    #[arg(short, long)]
    reading: Option<String>,

    /// Index of first image to read 
    #[arg(short, long)]
    index: Option<usize>,
}

const DEFAULT_DIR :&str  = "images/";
const ENV_VARIABLE :&str = "GALLSHDIR";

fn main() {

    let args = Args::parse();
    let gallshdir = env::var(ENV_VARIABLE);

    let selection_file = if let Some(selection_file_arg) = args.selection {
        String::from(selection_file_arg)
    } else  {
        String::from("./selected_files")
    };

    let path = if let Some(directory_arg) = args.directory {
        String::from(directory_arg)
    } else if let Ok(standard_dir) = gallshdir {
        String::from(standard_dir)
    } else {
        println!("GALLSHDIR variable not set. Using {} as default.", DEFAULT_DIR);
        String::from(DEFAULT_DIR)
    };

    let reading_list = &args.reading;

    let index_start = args.index;

    if let Some(reading_list_file) = reading_list {
        println!("searching images from the {} reading list", reading_list_file)
    } else {
        println!("searching images in {}", path)
    };

    // build an application with some css characteristics
    let application = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    application.connect_startup(|_| {
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data("window { background-color:black;} image { margin:1em ; }");
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css_provider,
            1000,
            );
    });

    let pattern = args.pattern;
    // clone! passes a strong reference to pattern in the closure that activates the application
    application.connect_activate(clone!(@strong reading_list, @strong pattern => move |application: &gtk::Application| { 


        // get all the filenames in the directory that match pattern (or all if None) or from a
        // reading list
        let mut filenames = if let Some(reading_list_filename) = &reading_list {
            match get_files_from_reading_list(reading_list_filename) {
                Err(msg) => panic!("{}", msg),
                Ok(result) => result,
            }
        } else {
            match get_files_in_directory(&path, &pattern) {
                Err(msg) => panic!("{}", msg),
                Ok(result) => result,
            }
        };

        filenames.sort();
        println!("{} files selected", filenames.len());
        if filenames.len() == 0 {
            application.quit();
            return
        }

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
        if let Some(index_number) = args.index {
            index.set(index_number);
        }
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
        // add an action to save this file reference
        let action = SimpleAction::new("save", None);
        action.connect_activate(clone!(@strong selection_file, @strong filenames, @strong index_rc => move |_, _| {
            let index = index_rc.get();
            let filename = format!("{}\n", &filenames[index.selected]);
            println!("saving reference {}.", filename);
            let save_file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(selection_file.clone());
            save_file.expect(&format!("could not open {}", selection_file)).write_all(filename.as_bytes());
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
    application.set_accels_for_action("win.save", &["s"]);
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn show_image(filenames: &Vec<String>, image: &Image, index_rc:&Rc<Cell<Index>>, window: &gtk::ApplicationWindow, navigate:Navigate) {
    let mut index = index_rc.get();
    match navigate {
        Navigate::Next => index.next(),
        Navigate::Prev => index.prev(),
        Navigate::Random => index.random(),
        Navigate::Current => { } ,
    }
    index_rc.set(index);
    let filename = &filenames[index.selected];
    image.set_from_file(Some(filename.clone()));
    window.set_title(Some(&format!("{} {}", index.selected, filename.as_str())));
}
