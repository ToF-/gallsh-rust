use glib::clone;
use glib::timeout_add_local;
use gtk::prelude::*;
use gtk::{self, Application, ScrolledWindow, gdk, glib, Grid, Picture};
use rand::{thread_rng, Rng};
use std::cell::{RefCell, RefMut};
use std::env;
use std::io;
use std::fs;
use std::fs::OpenOptions;
use std::fs::read_to_string;
use std::io::{Write};
use std::rc::Rc;
use std::time::{Duration};
use std::time::SystemTime;
use clap::Parser;
use walkdir::WalkDir;

#[derive(Clone, Debug)]
struct Entry {
    file_path: String,
    file_size: u64,
    modified_time: SystemTime,
    to_select: bool,
    to_touch: bool,
    to_unlink: bool,
}

type EntryList = Vec<Entry>;

fn make_entry(s:String, l:u64, t:SystemTime) -> Entry {
    return Entry { 
      file_path: s.clone(),
      file_size: l,
      modified_time: t,
      to_select: false,
      to_touch: false,
      to_unlink: false,
    }
}

// a struct to keep track of navigating in image files
#[derive(Clone, Debug)]
struct Index {
    entries: Vec<Entry>,
    current: usize,
    maximum:  usize,
    start_index: usize,
    grid_size: usize,
    real_size: bool,
    register: usize,
}

impl Index {
    fn new(entries: Vec<Entry>, grid_size: usize) -> Self {
        Index {
            entries: entries.clone(),
            current: 0,
            maximum: entries.len() - 1,
            start_index: 0,
            grid_size: grid_size,
            real_size: false,
            register: 0,

        }
    }
    fn selection_size(self) -> usize {
        return self.grid_size * self.grid_size
    }

    fn next(&mut self) {
        let selection_size = self.clone().selection_size();
        let next_pos = (self.current + selection_size) % (self.maximum + 1);
        self.current = if self.current < self.maximum { next_pos } else { 0 } ;
        self.register = 0;

    }
    fn prev(&mut self) {
        let selection_size = self.clone().selection_size();
        let next_pos = self.current - selection_size;
        self.current = if next_pos <= self.maximum { next_pos } else { self.maximum - selection_size };
        self.register = 0;
    }

    fn random(&mut self) {
        self.current = thread_rng().gen_range(0..self.maximum + 1);
        self.register = 0;
    }
    fn set(&mut self, value: usize) {
        if value < self.maximum {
            self.current = value;
        } else {
            println!("index {} out of range, set to 0", value);
            self.current = 0;
        }
    }

    fn current_filename(self) -> String {
        return self.entries[self.current].file_path.clone()
    }
    fn nth_filename(self, i: usize) -> String {
        if self.current + i <= self.maximum {
            return self.entries[self.current + i].file_path.clone()
        } else {
            return self.entries[self.current + i - self.maximum].file_path.clone()
        }
    }

    fn register_digit(&mut self, s:&str) {
        let digit:usize = s.parse().unwrap();
        self.register = self.register * 10 + digit;
    }

    fn set_register(&mut self) {
        self.set(self.register);
        self.register = 0;
    }

    fn start_area(&mut self) {
        self.start_index = self.current
    }

    fn toggle_real_size(&mut self) {
        self.real_size = !self.real_size;
    }

    fn toggle_to_select(&mut self, index: usize) {
        self.entries[index].to_select = ! self.entries[index].to_select;
    }

    fn toggle_to_select_current(&mut self) {
        self.entries[self.current].to_select = ! self.entries[self.current].to_select;
    }

    fn toggle_to_unlink_current(&mut self) {
        self.entries[self.current].to_unlink = ! self.entries[self.current].to_unlink;
    }

    fn toggle_to_touch_current(&mut self) {
        self.entries[self.current].to_touch = ! self.entries[self.current].to_touch;
    }

    fn save_marked_file_list(&mut self, selection: Vec<&Entry>, dest_file_path: &str) {
        if selection.len() > 0 {
            let result = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(dest_file_path);
            if let Ok(mut file) = result {
                for e in selection.iter() {
                    println!("saving {} for reference", e.file_path);
                    let _ = file.write(format!("{}\n", e.file_path).as_bytes());
                }
            }
        }
    }

    fn save_marked_file_lists(&mut self) {
        let entries = &self.entries.clone();
        let _ = &self.save_marked_file_list(entries.iter().filter(|e| e.to_select).collect(), "selections");
        let _ = &self.save_marked_file_list(entries.iter().filter(|e| e.to_touch).collect(), "touches");
        let _ = &self.save_marked_file_list(entries.iter().filter(|e| e.to_unlink).collect(), "deletions");
    }
}

#[derive(PartialEq)]
enum Navigate {
    Current,
    Next,
    Prev,
    Random,
}

fn file_name(entry: &Entry) -> &str {
    return &entry.file_path
}

fn file_size(entry: &Entry) -> u64 {
    return entry.file_size
}

fn file_modified_time(entry: &Entry) -> SystemTime {
    return entry.modified_time
}

fn get_files_from_reading_list(reading_list: &String) -> io::Result<EntryList> {
    match read_to_string(reading_list) {
        Ok(content) => {
            let mut entries: EntryList = Vec::new();
            for file_name in content.lines().map(String::from).collect::<Vec<_>>() {
                let metadata = fs::metadata(&file_name)?;
                let file_size = metadata.len();
                let entry_name = file_name.to_string().to_owned();
                let modified_time = metadata.modified().unwrap();
            entries.push(make_entry(entry_name, file_size, modified_time));
            };
            Ok(entries)
        },
        Err(msg) => Err(msg)
    }

}
fn get_files_in_directory(dir_path: &str, opt_pattern: &Option<String>, opt_low_size: Option<u64>, opt_high_size: Option<u64>) -> io::Result<EntryList> {
    let mut entries: EntryList = Vec::new();
    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.into_path();
        let valid_ext = if let Some(ext) = path.extension() {
            ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "JPG" || ext == "JPEG" || ext == "PNG"
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
            let file_size = metadata.len();
            let modified_time = metadata.modified().unwrap();
            if low_size_limit <= file_size && file_size <= high_size_limit  {
                if let Some(full_name) = path.to_str() {
                    let entry_name = full_name.to_string().to_owned();
                    entries.push(make_entry(entry_name, file_size, modified_time));
                }
            }
        }
    };
    Ok(entries)
}

// declarative setting of arguments
/// Gallery Show
#[derive(Parser, Debug)]
#[command(infer_subcommands = true, infer_long_args = true, author, version, about, long_about = None)]
/// Pattern that displayed files must have
struct Args {
    #[arg(short, long)]
    pattern: Option<String>,

    /// Maximized window
    #[arg(short, long, default_value_t = false)]
    maximized: bool,

    /// Ordered display (or random)
    #[arg(short, long)]
    ordered: Option<char>,

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


        // get all the entries in the directory that match pattern (or all if None) or from a
        // reading list
        let mut entry_list = if let Some(reading_list_filename) = &reading_list {
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

        if let Some(order) = args.ordered {
            match order {
                's' => entry_list.sort_by(|a, b| { file_size(&a).cmp(&file_size(&b)) }),
                'S' => entry_list.sort_by(|a, b| { file_size(&b).cmp(&file_size(&a)) }),
                'd' => entry_list.sort_by(|a, b| { file_modified_time(&a).cmp(&file_modified_time(&b)) }),
                'U' => entry_list.sort_by(|a, b| { file_modified_time(&b).cmp(&file_modified_time(&a)) }),
                _ => entry_list.sort_by(|a, b| { file_name(&a).cmp(file_name(&b)) }),
            }
        }

        println!("{} files selected", entry_list.len());
        if entry_list.len() == 0 {
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

        let mut index = Index::new(entry_list.clone(), grid_size);
        if let None = args.ordered {
            index.random()
        };
        if let Some(index_number) = args.index {
            index.set(index_number);
        }
        let index_rc = Rc::new(RefCell::new(index));

        let entries_rc: Rc<RefCell<EntryList>> = Rc::new(RefCell::new(entry_list));

        // handle key events
        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(clone!(@strong entries_rc, @strong grid, @strong index_rc, @strong window => move |_, key, _, _| {
            let step = 100;
            let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
            if let Some(s) = key.name() {
                match s.as_str() {
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let digit:usize = s.parse().unwrap();
                        index.register = index.register * 10 + digit;
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "g" => {
                        index.set_register();
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "j" => {
                        for _ in 0..10 {
                            index.next()
                        }
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "b" => {
                        for _ in 0..10 {
                            index.prev()
                        }
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "f" => {
                        if (index.clone().selection_size()) == 1 {
                            index.toggle_real_size();
                        }
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "z" => {
                        index.set(0);
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    }
                    "n" => {
                        index.next();
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    }
                    "p" => {
                        index.prev();
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    }
                    "q" => {
                        index.save_marked_file_lists();
                        window.close();
                        gtk::Inhibit(true)
                    },
                    "r" => {
                        index.random();
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "s" => {
                        index.toggle_to_select_current();
                        index.toggle_to_select_current();
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "t" => {
                        index.toggle_to_touch_current();
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "u" => { 
                        index.toggle_to_unlink_current();
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
                    },
                    "a" => {
                        index.start_area();
                        gtk::Inhibit(false)
                    },
                    "e" => {
                        if index.current >= index.start_index {
                            for i in index.start_index .. index.current+1 {
                                index.toggle_to_select(i);
                            }
                        } else {
                            println!("area start index {} is greater than area end index {}", index.start_index, index.current);
                        }
                        gtk::Inhibit(false)
                    },
                    "space" => { 
                        if let Some(_) = args.ordered { 
                            index.next()
                        } else {
                            index.random()
                        }
                        show_grid_alt(&grid, index.clone(), &window);
                        gtk::Inhibit(false)
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
        if let Some(_) = args.ordered {
            let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
            show_grid_alt(&grid, index.clone(), &window);
        } else {
            let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
            index.random();
            show_grid_alt(&grid, index.clone(), &window);
        }

        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong entries_rc, @strong grid, @strong index_rc, @strong window => move | | { 
                if let Some(_) = args.ordered { 
                    show_grid(&grid, &index_rc, &window, Navigate::Next)
                } else {
                    show_grid(&grid, &index_rc, &window, Navigate::Random)
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


fn show_marks(entry: &Entry) -> String {
    format!("{}|{}|{}",
        if entry.to_select { "SELECT" } else { "" },
        if entry.to_touch { "TOUCH" } else { "" },
        if entry.to_unlink { "UNLINK" } else { "" }).clone()
}

fn save_marked_file_lists(index_rc:&Rc<RefCell<Index>>) {
    let mut index = index_rc.borrow_mut();
    index.save_marked_file_lists();
}
fn show_grid(grid: &Grid, index_rc:&Rc<RefCell<Index>>, window: &gtk::ApplicationWindow, navigate:Navigate) {
    let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
    let entries = index.entries.clone();
    let selection_size = index.clone().selection_size();
    match navigate {
        Navigate::Next => index.next(),
        Navigate::Prev => index.prev(),
        Navigate::Random => index.random(),
        Navigate::Current => { } ,
    }
    for i in 0 .. selection_size {
        let row = (i / index.grid_size) as i32;
        let col = (i % index.grid_size) as i32;
        let picture = grid.child_at(col,row).unwrap().downcast::<gtk::Picture>().unwrap();
        let current = if navigate != Navigate::Random || selection_size == 1 {
            index.current + i
        } else {
            thread_rng().gen_range(0..index.maximum + 1)
        };
        if current <= index.maximum {
            let filename = index.clone().current_filename();
            picture.set_can_shrink(!index.real_size);
            picture.set_filename(Some(filename));
        }
    }
    window.set_title(Some(&format!("{} {} {} [{}] {}",
                index.current,
                &entries[index.current].file_path.as_str(),
                show_marks(&entries[index.current]),
                index.register,
                if index.real_size { "*" } else { ""} )));
}
fn show_grid_alt(grid: &Grid, mut index: Index, window: &gtk::ApplicationWindow) {
    let entries = index.entries.clone();
    let selection_size = index.clone().selection_size();
    for i in 0 .. selection_size {
        let row = (i / index.grid_size) as i32;
        let col = (i % index.grid_size) as i32;
        let picture = grid.child_at(col,row).unwrap().downcast::<gtk::Picture>().unwrap();
        let filename = index.clone().nth_filename(i);
        picture.set_can_shrink(!index.real_size);
        picture.set_filename(Some(filename));
    }
    window.set_title(Some(&format!("{} {} {} [{}] {}",
                index.current,
                &entries[index.current].file_path.as_str(),
                show_marks(&entries[index.current]),
                index.register,
                if index.real_size { "*" } else { ""} )));
}
