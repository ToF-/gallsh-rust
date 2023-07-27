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
        println!("selected:{}", self.selected);

    }

    fn prev(&mut self) {
        self.selected = if self.selected > 0 { self.selected - 1 } else { self.maximum };
        println!("selected:{}", self.selected);
    }

    fn random(&mut self) {
        self.selected = thread_rng().gen_range(0..self.maximum + 1);
        println!("selected:{}", self.selected);
    }

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
}

fn main() {
    // acquire the image directory from env variable
    let mut gallery_show_dir = String::from("images/");
    match env::var("GALLSHDIR")  {
        Ok(val) => gallery_show_dir = String::from(val),
        Err(e) => {
            println!("GALLSHDIR: {e}\n default to \"{gallery_show_dir}\"");
        }
    };
    // parse the command line arguments arguments
    let args = Args::parse();

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
        let mut filenames = match get_files_in_directory(&gallery_show_dir, &pattern) {
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
            println!("{:?}", index_rc);
            window.close();
        }));
        window.add_action(&action_close);

        // add an action to show random image
        let action = SimpleAction::new("random", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            let mut index = index_rc.get();
            index.random();
            index_rc.set(index);
            show_image(&filenames, &image, index.selected, window);
        }));
        window.add_action(&action);

        // add an action to show next image
        let action = SimpleAction::new("next", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            let mut index = index_rc.get();
            index.next();
            index_rc.set(index);
            show_image(&filenames, &image, index.selected, window);
        }));
        window.add_action(&action);
        
        // add an action to show prev image
        let action = SimpleAction::new("prev", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            let mut index = index_rc.get();
            index.prev();
            index_rc.set(index);
            show_image(&filenames, &image, index.selected, window);
        }));
        window.add_action(&action);

        // add an action to show next or random image
        let action = SimpleAction::new("randnext", None);
        action.connect_activate(clone!(@strong filenames, @strong image, @strong index_rc, @weak window => move |_, _| {
            let mut index = index_rc.get();
            if args.ordered { index.next() } else { index.random() };
            index_rc.set(index);
            show_image(&filenames, &image, index.selected, window);
        }));
        window.add_action(&action);
        // show the first file
        let filename = &filenames[index_rc.get().selected];
        image.set_from_file(Some(filename.clone()));
        window.set_title(Some(filename.as_str()));

        if args.maximized { window.fullscreen() };
        window.present();
    }));
    application.set_accels_for_action("win.close", &["q"]);
    application.set_accels_for_action("win.random", &["r"]);
    application.set_accels_for_action("win.next", &["n"]);
    application.set_accels_for_action("win.prev", &["p"]);
    let empty: Vec<String> = vec![];

    // run the application with empty args as the have been parsed before app creation
    application.run_with_args(&empty);
}

fn show_image(filenames: &Vec<String>, image: &Image, index:usize, window: gtk::ApplicationWindow) {
    let filename = &filenames[index];
    image.set_from_file(Some(filename.clone()));
    window.set_title(Some(filename.as_str()));
}
