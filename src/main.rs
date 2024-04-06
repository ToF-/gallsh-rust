use gio::SimpleAction;
use glib::clone;
use glib::timeout_add_local;
use gtk::prelude::*;
use gtk::{self, Application, ScrolledWindow, gdk, gio, glib, Grid, Image, Picture};
use rand::{thread_rng, Rng};
use std::cell::Cell;
use std::env;
use std::io;
use std::fs;
use std::fs::OpenOptions;
use std::fs::read_to_string;
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
    start_index: usize,
    grid_size: usize,
    real_size: bool,
}

impl Index {
    fn new(capacity: usize, grid_size: usize) -> Self {
        Index {
            selected: 0,
            maximum: capacity - 1,
            start_index: 0,
            grid_size: grid_size,
            real_size: false,
        }
    }

    fn selection_size(self) -> usize {
        self.grid_size * self.grid_size 
    }

    fn next(&mut self) {
        self.selected = if self.selected < self.maximum { (self.selected + self.selection_size()) % (self.maximum + 1) } else { 0 } ;

    }
    fn prev(&mut self) {
        self.selected = if self.selected > 0 { self.selected - 1 } else { self.maximum };
    }

    fn random(&mut self) {
        self.selected = thread_rng().gen_range(0..self.maximum + 1);
    }
    fn set(&mut self, value: usize) {
        if value < self.maximum {
            self.selected = value;
        } else {
            println!("index {} out of range, set to 0", value);
            self.selected = 0;
        }
    }

    fn start_area(&mut self) {
        self.start_index = self.selected
    }

    fn toggle_real_size(&mut self) {
        self.real_size = !self.real_size;
    }
}

#[derive(PartialEq)]
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
fn get_files_in_directory(dir_path: &str, opt_pattern: &Option<String>, opt_low_size: Option<u64>, opt_high_size: Option<u64>) -> io::Result<Vec<String>> {
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
        let low_size_limit = if let Some(low) = opt_low_size {
            low
        } else {
            0
        };
        let high_size_limit = if let Some(high) = opt_high_size {
            high
        } else {
            std::u64::MAX
        };
        if valid_ext && pattern_present {
            let metadata = fs::metadata(&path)?;
            let len = metadata.len();
            if low_size_limit <= len && len <= high_size_limit  {
                if let Some(full_name) = path.to_str() {
                    file_names.push(full_name.to_string().to_owned());
                }
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

    /// Grid Size
    #[arg(short, long)]
    grid: Option<usize>,

    /// Low Limit on file size
    #[arg(short, long)]
    low: Option<u64>,

    /// High Limit on file size
    #[arg(short, long)]
    high: Option<u64>,
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

    let grid_size = if let Some(size) = args.grid { if size >= 2 && size <= 10 { size } else { 1 } } else { 1 };

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
            match get_files_in_directory(&path, &pattern, args.low, args.high) {
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
        let grid = Grid::new();
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(true);
        grid.set_hexpand(true);
        grid.set_vexpand(true);
        for row in 0 .. grid_size {
            for col in 0 .. grid_size {
                let image = Picture::new();
                grid.attach(&image, row as i32, col as i32, 1, 1);
            }
        }
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .default_width(1000)
            .default_height(1000)
            .build();

        let scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        scrolled_window.set_child(Some(&grid));
        window.set_child(Some(&scrolled_window));

        let mut index = Index::new(filenames.len(), grid_size);
        if !args.ordered {
            index.random()
        };
        if let Some(index_number) = args.index {
            index.set(index_number);
        }
        let index_rc = Rc::new(Cell::new(index));


        // handle key events
        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(clone!(@strong selection_file, @strong filenames, @strong grid, @strong index_rc, @strong window => move |_, key, _, _| {
            let step = 100;
            if let Some(s) = key.name() {
                match s.as_str() {
                    "a" => start_references(&filenames, &index_rc),
                    "b" => jump_back_ten(&filenames, &grid, &index_rc, &window),
                    "e" => end_references(&selection_file, &filenames, &index_rc),
                    "f" => toggle_full_size(&filenames, &grid, &index_rc, &window),
                    "j" => jump_forward_ten(&filenames, &grid, &index_rc, &window),
                    "n" => {
                        show_grid(&filenames, &grid, &index_rc, &window, Navigate::Next);
                        gtk::Inhibit(false)
                    }
                    "p" => {
                        show_grid(&filenames, &grid, &index_rc, &window, Navigate::Prev);
                        gtk::Inhibit(false)
                    }
                    "q" => {
                        window.close();
                        gtk::Inhibit(true)
                    },
                    "r" => {
                        show_grid(&filenames, &grid, &index_rc, &window, Navigate::Random);
                        gtk::Inhibit(false)
                    },
                    "s" => save_reference(&selection_file, &filenames, &index_rc),

                    "space" => { 
                        if args.ordered { 
                            show_grid(&filenames, &grid, &index_rc, &window, Navigate::Next);
                            gtk::Inhibit(false)
                        } else {
                            show_grid(&filenames, &grid, &index_rc, &window, Navigate::Random);
                            gtk::Inhibit(false)
                        }
                    },
                    "Right" => {
                        let h_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.hadjustment()))
                            .expect("Failed to get hadjustment");
                        h_adj.set_value(h_adj.value() + step as f64);
                        gtk::Inhibit(true)
                    },
                    "Left" => {
                        let h_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.hadjustment()))
                            .expect("Failed to get hadjustment");
                        h_adj.set_value(h_adj.value() - step as f64);
                        gtk::Inhibit(true)
                    },
                    "Down" => {
                        // Scroll down
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() + step as f64);
                        gtk::Inhibit(true)
                    },
                    "Up" => {
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() - step as f64);
                        gtk::Inhibit(true)
                    }
                    _ => { gtk::Inhibit(false)},
                }
            }
            else {
                gtk::Inhibit(false)
            }
        }));
        window.add_controller(evk);
        // show the first file
        if args.ordered {
            show_grid(&filenames, &grid, &index_rc, &window, Navigate::Current);
        } else {
            show_grid(&filenames, &grid, &index_rc, &window, Navigate::Random);
        }

        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong filenames, @strong grid, @strong index_rc, @strong window => move | | { 
                if args.ordered { 
                    show_grid(&filenames, &grid, &index_rc, &window, Navigate::Next)
                } else {
                    show_grid(&filenames, &grid, &index_rc, &window, Navigate::Random)
                };
                Continue(true) 
            }));
    };
        window.present();
    }));
    application.set_accels_for_action("win.save", &["s"]);
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn save_reference(selection_file: &String, filenames: &Vec<String>, index_rc: &Rc<Cell<Index>>) -> gtk::Inhibit {
    let index = index_rc.get();
    let filename = format!("{}\n", &filenames[index.selected]);
    println!("saving reference {}.", filename);
    let save_file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(selection_file.clone());
    save_file.expect(&format!("could not open {}", selection_file)).write_all(filename.as_bytes());
    gtk::Inhibit(true)
}

fn start_references(filenames: &Vec<String>, index_rc: &Rc<Cell<Index>>) -> gtk::Inhibit {
    let mut index = index_rc.get();
    let filename = format!("{}\n", &filenames[index.selected]);
    index.start_area();
    println!("starting saving references from {}.", filename);
    index_rc.set(index);
    gtk::Inhibit(true)
}

fn end_references(selection_file: &String, filenames: &Vec<String>, index_rc: &Rc<Cell<Index>>) -> gtk::Inhibit {
    let index = index_rc.get();
    if index.selected >= index.start_index {
        for i in index.start_index .. index.selected+1 {
            let filename = format!("{}\n", &filenames[i]);
            println!("saving reference {}", filename);
            let save_file = OpenOptions::new().write(true).append(true).create(true)
                .open(selection_file.clone());
            save_file.expect(&format!("could not open {}", selection_file)).write_all(filename.as_bytes());
        }
    } else {
        println!("area start index {} is greater than area end index {}", index.start_index, index.selected);
    }
    gtk::Inhibit(true)
}

fn jump_forward_ten(filenames: &Vec<String>,  grid: &Grid, index_rc:&Rc<Cell<Index>>, window: &gtk::ApplicationWindow) -> gtk::Inhibit {
    let mut index = index_rc.get();
    for _ in 0..9 {
        index.next()
    };
    index_rc.set(index);
    show_grid(&filenames, &grid, &index_rc, &window, Navigate::Next);
    gtk::Inhibit(false)
}

fn jump_back_ten(filenames: &Vec<String>,  grid: &Grid, index_rc:&Rc<Cell<Index>>, window: &gtk::ApplicationWindow) -> gtk::Inhibit {
    let mut index = index_rc.get();
    for _ in 0..9 {
        index.prev()
    };
    index_rc.set(index);
    show_grid(&filenames, &grid, &index_rc, &window, Navigate::Prev);
    gtk::Inhibit(false)
}

fn toggle_full_size(filenames: &Vec<String>, grid: &Grid, index_rc: &Rc<Cell<Index>>, window: &gtk::ApplicationWindow) -> gtk::Inhibit {
    let mut index = index_rc.get();
    if index.selection_size() == 1 {
        index.toggle_real_size();
        index_rc.set(index);
        show_grid(filenames, grid, index_rc, window, Navigate::Current);
        gtk::Inhibit(false)
    } else {
        gtk::Inhibit(true)
    }
}

fn show_grid(filenames: &Vec<String>, grid: &Grid, index_rc:&Rc<Cell<Index>>, window: &gtk::ApplicationWindow, navigate:Navigate) {
    let mut index = index_rc.get();
    match navigate {
        Navigate::Next => index.next(),
        Navigate::Prev => index.prev(),
        Navigate::Random => index.random(),
        Navigate::Current => { } ,
    }
    index_rc.set(index);
    let filename = &filenames[index.selected];
    for i in 0 .. (index.selection_size()) {
        let row = (i / index.grid_size) as i32;
        let col = (i % index.grid_size) as i32;
        let picture = grid.child_at(col,row).unwrap().downcast::<gtk::Picture>().unwrap();
        let selected = if navigate != Navigate::Random || index.selection_size() == 1 {
            index.selected + i
        } else {
            thread_rng().gen_range(0..index.maximum + 1)
        };
        if selected <= index.maximum {
            let filename = &filenames[selected];
            picture.set_can_shrink(!index.real_size);
            picture.set_filename(Some(filename.clone()));
        }
    }
    if index.selection_size() == 1 {
        window.set_title(Some(&format!("{} {}", index.selected, filename.as_str())));
    }
}
