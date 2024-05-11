use mime;
use clap::{Parser,ValueEnum};
use clap_num::number_range;
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::EventControllerMotion;
use gtk::prelude::*;
use gtk::{self, Application, ScrolledWindow, gdk, glib, Grid, Picture};
use rand::{thread_rng, Rng};
use std::cell::{RefCell, RefMut};
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::fs::read_to_string;
use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::io::{Error,ErrorKind};
use std::io::{Write};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;
use std::time::{Duration};
use thumbnailer::error::{ThumbResult, ThumbError};
use thumbnailer::{create_thumbnails, ThumbnailSize};
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

// a struct to keep track of navigating in a list of image files
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
        let next_pos = if self.current >= selection_size { self.current - selection_size } else { self.maximum - selection_size + 1 };
        self.current = next_pos;
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

    fn nth_filename(self, i: usize) -> String {
        self.entries[self.clone().nth_index(i)].file_path.clone()
    }

    fn nth_index(self, i: usize) -> usize {
        (self.current + i) % (self.maximum + 1)
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
        if index <= self.maximum {
            self.entries[index].to_select = ! self.entries[index].to_select;
        } else {
            println!("index out of range: {}/{}", index, self.maximum);
        }
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

fn get_files_from_reading_list(reading_list: &String) -> io::Result<EntryList> {
    match read_to_string(reading_list) {
        Ok(content) => {
            let mut entries: EntryList = Vec::new();
            let mut filenames: HashSet<String> = HashSet::new();
            for path in content.lines().map(String::from).collect::<Vec<_>>() {
                match fs::metadata(&path) {
                    Ok(metadata) => {
                        let file_size = metadata.len();
                        let entry_name = path.to_string().to_owned();
                        let modified_time = metadata.modified().unwrap();
                        if ! filenames.contains(&entry_name) {
                            filenames.insert(entry_name.clone());
                            entries.push(make_entry(entry_name, file_size, modified_time));
                        }
                    }
                    Err(err) => {
                        println!("can't open: {}: {}", path, err);
                    }
                }
            };
            Ok(entries)
        },
        Err(msg) => Err(msg)
    }

}

fn create_thumbnail(source: String, target: String, number: Option<usize>) -> ThumbResult<()> {
    if let Some(n) = number {
        println!("{:6} {}", n, target.clone())
    } else {
        println!("{}", target.clone())
    };
    match File::open(source.clone()) {
        Err(err) => {
            println!("error opening file {}: {}", source, err);
            return Err(ThumbError::IO(err))
        },
        Ok(input_file) => {
            let source_path = Path::new(source.as_str());
            let source_extension = match source_path.extension().and_then(OsStr::to_str) {
                None => {
                    println!("error: file {} has no extension", source.clone());
                    return Err(ThumbError::IO(Error::new(ErrorKind::Other, "no extension")))
                },
                Some(s) => s,
            };
            let reader = BufReader::new(input_file);
            let output_file = match File::create(target.clone()) {
                Err(err) => {
                    println!("error while creating file {}: {}",
                        target.clone(),
                        err);
                    return Err(ThumbError::IO(err))
                },
                Ok(file) => file,
            };
            write_thunbnail(reader, source_extension, output_file)
        },
    }
}
fn write_thunbnail<R: std::io::Seek + std::io::Read>(reader: BufReader<R>, extension: &str, mut output_file: File) -> ThumbResult<()> {
    let mime = match extension {
        "jpg" | "jpeg" | "JPG" | "JPEG" => mime::IMAGE_JPEG,
        "png" | "PNG" => mime::IMAGE_PNG,
        _ => panic!("wrong extension"),
    };
    let mut thumbnails = match create_thumbnails(reader, mime, [ThumbnailSize::Small]) {
        Ok(tns) => tns,
        Err(err) => {
            println!("error while creating thumbnails:{:?}", err);
            return Err(err)
        },
    };
    let thumbnail = thumbnails.pop().unwrap();
    let write_result = match extension {
        "jpg" | "jpeg" | "JPG" | "JPEG" => thumbnail.write_jpeg(&mut output_file,255),
        "png" | "PNG" => thumbnail.write_png(&mut output_file),
        _ => panic!("wrong extension"),
    };
    match write_result {
        Err(err) => {
            println!("error while writing ihunbnail:{}", err);
            Err(err)
        },
        ok => ok,
    }
}
fn update_thumbnails(dir_path: &str) -> ThumbResult<usize> {
    let result_images = get_files_in_directory(dir_path, false, &None, None, None);
    let image_entries = match result_images {
        Ok(entries) => entries,
        Err(err) => return Err(ThumbError::IO(err)),
    };
    let result_thumbnails = get_files_in_directory(dir_path, true, &None, None, None);
    let mut thumbnail_entries = match result_thumbnails {
        Ok(entries) => entries,
        Err(err) => return Err(ThumbError::IO(err)),
    };
    thumbnail_entries.sort_by(|a, b| { a.file_path.cmp(&b.file_path) });
    let mut number: usize = 0;
    for entry in image_entries {
            let image_path: PathBuf = PathBuf::from(entry.file_path);
            let mut target_path: PathBuf = image_path.clone();
            let extension = target_path.extension().unwrap();
            let file_stem = target_path.file_stem().unwrap();
            let new_file_name = format!("{}THUMB.{}",
                file_stem.to_str().unwrap(),
                extension.to_str().unwrap());
            target_path.set_file_name(new_file_name);
            let source = image_path.into_os_string().into_string().unwrap();
            let target = target_path.into_os_string().into_string().unwrap();
            if let Err(_) = thumbnail_entries.binary_search_by(|probe|
                probe.file_path.cmp(&target)) {
                let _ = create_thumbnail(source, target, Some(number));
                number += 1;
            } else {
            }
    };
    Ok(0)
}

fn get_file(file_path: &str) -> io::Result<EntryList> {
    let mut entries: EntryList = Vec::new();
    if let Ok(metadata) = fs::metadata(&file_path) {
        let file_size = metadata.len();
        let modified_time = metadata.modified().unwrap();
        let entry_name = file_path.to_string().to_owned();
        entries.push(make_entry(entry_name, file_size, modified_time));
    } else {
        println!("can't open: {}", file_path);
    };
    Ok(entries)
}
fn get_files_in_directory(dir_path: &str, thumbnails_only: bool, opt_pattern: &Option<String>, opt_low_size: Option<u64>, opt_high_size: Option<u64>) -> io::Result<EntryList> {
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
        let check_thumbnails = pattern_present && path.to_str().map(|filename| filename.contains("THUMB")) == Some(thumbnails_only);
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
        if valid_ext && pattern_present && check_thumbnails {
            if let Ok(metadata) = fs::metadata(&path) {
                let file_size = metadata.len();
                let modified_time = metadata.modified().unwrap();
                if low_size_limit <= file_size && file_size <= high_size_limit  {
                    if let Some(full_name) = path.to_str() {
                        let entry_name = full_name.to_string().to_owned();
                        entries.push(make_entry(entry_name, file_size, modified_time));
                    }
                }
            } else {
                println!("can't open: {}", path.display());
            }
        }
    };
    Ok(entries)
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Order {
    Name, Size, Date,
}

#[derive(Debug, PartialEq, Eq)]
struct ParseOrderError;

fn less_than_11(s: &str) -> Result<usize, String> {
    number_range(s,1,10)
}

// declarative setting of arguments
/// Gallery Show
#[derive(Parser, Clone, Debug)]
#[command(infer_subcommands = true, infer_long_args = true, author, version, about, long_about = None)]
/// Pattern that displayed files must have
struct Args {
    #[arg(short, long)]
    pattern: Option<String>,

    /// Maximized window
    #[arg(short, long, default_value_t = false)]
    maximized: bool,

    /// Ordered display (or random)
    #[arg(short, long,value_name("order"),value_parser(clap::value_parser!(Order)))]
    ordered: Option<Order>,

    /// Timer delay for next picture
    #[arg(long)]
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
    #[arg(short, long, value_parser=less_than_11)]
    grid: Option<usize>,

    /// Low Limit on file size
    #[arg(short, long)]
    low: Option<u64>,

    /// High Limit on file size
    #[arg(short, long)]
    high: Option<u64>,

    /// File to view
    #[arg(short, long)]
    file: Option<String>, 

    /// Thumbnails only
    #[arg(long)]
    thumbnails: bool,

    /// Update thumbnails before showing files
    #[arg(long)]
    update_thumbnails: bool,
}

const DEFAULT_DIR :&str  = "images/";
const ENV_VARIABLE :&str = "GALLSHDIR";

fn main() {

    let args = Args::parse();
    let gallshdir = env::var(ENV_VARIABLE);

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

    // clone! passes a strong reference to pattern in the closure that activates the application
    application.connect_activate(clone!(@strong args => move |application: &gtk::Application| { 

        let order: Order = if let Some(result) = args.clone().ordered { result } else { Order::Name };
        let pattern = &args.pattern;
        let path = if let Some(directory_arg) = &args.directory {
            String::from(directory_arg)
        } else if let Ok(standard_dir) = &gallshdir {
            String::from(standard_dir)
        } else {
            println!("GALLSHDIR variable not set. Using {} as default.", DEFAULT_DIR);
            String::from(DEFAULT_DIR)
        };

        let reading_list = &args.reading;

        let grid_size = if args.thumbnails && args.grid == None {
            10
        } else {
            if let Some(size) = args.grid { 
                if size <= 10 {
                    size
                } else {
                    if args.thumbnails { 10 } else { 1 }
                }
            } else { 1 }
        };

        if let Some(reading_list_file) = reading_list {
            println!("searching images from the {} reading list", reading_list_file)
        } else {
            println!("searching images in {}", path)
        };


        if args.update_thumbnails {
            println!("updating thumbnails...");
            if let Ok(n) = update_thumbnails(&path) {
                println!("{n} thumbnails added");
            }
        }
        // get all the entries in the directory that match pattern (or all if None) or from a
        // reading list
        let mut entry_list = if let Some(reading_list_filename) = &reading_list {
            match get_files_from_reading_list(reading_list_filename) {
                Err(msg) => panic!("{}", msg),
                Ok(result) => result,
            }
        } else {
            if let Some(file) = &args.file {
                match get_file(file) {
                    Err(msg) => panic!("{}", msg),
                    Ok(result) => result,
                } 
            } else {
                match get_files_in_directory(&path, args.thumbnails, &pattern, args.low, args.high) {
                    Err(msg) => panic!("{}", msg),
                    Ok(result) => result,
                }
            }
        };
        match order {
            Order::Size => entry_list.sort_by(|a, b| { a.file_size.cmp(&b.file_size) }),
            Order::Date => entry_list.sort_by(|a, b| { a.modified_time.cmp(&b.modified_time) }),
            Order::Name => entry_list.sort_by(|a, b| { a.file_path.cmp(&b.file_path) }),
        }

        println!("{} files selected", entry_list.len());
        if entry_list.len() == 0 {
            application.quit();
            return
        }

        let mut index = Index::new(entry_list.clone(), grid_size);
        if let None = args.ordered {
            index.random()
        };
        if let Some(index_number) = args.index {
            index.set(index_number);
        }
        let index_rc = Rc::new(RefCell::new(index));

        let entries_rc: Rc<RefCell<EntryList>> = Rc::new(RefCell::new(entry_list));


        // build the main window
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .default_width(1000)
            .default_height(1000)
            .build();

        let scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let grid = Grid::new();
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(true);
        grid.set_hexpand(true);
        grid.set_vexpand(true);
        for row in 0 .. grid_size {
            for col in 0 .. grid_size {
                let image = Picture::new();
                grid.attach(&image, row as i32, col as i32, 1, 1);
                let gesture = gtk::GestureClick::new();
                gesture.set_button(0);
                gesture.connect_pressed(clone!(@strong index_rc, @strong grid, @strong window => move |_,_, _, _| {
                    let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
                    let entry_index = index.clone().nth_index(col * grid_size + row);
                    index.toggle_to_select(entry_index);
                    show_grid(&grid, index.clone(), &window);
                }));
                image.add_controller(gesture);
                let motion_controller = EventControllerMotion::new(); 
                motion_controller.connect_enter(clone!(@strong index_rc => move |_,_,_| {
                    let index: RefMut<'_,Index> = index_rc.borrow_mut();
                    let entry_index = index.clone().nth_index(col * grid_size + row);
                    let filename = <Index as Clone>::clone(&index).nth_filename(entry_index);
                    println!("{}", filename);
                }));
                image.add_controller(motion_controller)
            }
        }
        scrolled_window.set_child(Some(&grid));
        window.set_child(Some(&scrolled_window));

        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(clone!(@strong entries_rc, @strong grid, @strong index_rc, @strong window => move |_, key, _, _| {
            let step = 100;
            let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
            if let Some(s) = key.name() {
                match s.as_str() {
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let digit:usize = s.parse().unwrap();
                        index.register = index.register * 10 + digit;
                        show_grid(&grid, index.clone(), &window);
                    },
                    "g" => {
                        index.set_register();
                        show_grid(&grid, index.clone(), &window);
                    },
                    "j" => {
                        for _ in 0..10 {
                            index.next()
                        }
                        show_grid(&grid, index.clone(), &window);
                    },
                    "b" => {
                        for _ in 0..10 {
                            index.prev()
                        }
                        show_grid(&grid, index.clone(), &window);
                    },
                    "f" => {
                        if (index.clone().selection_size()) == 1 {
                            index.toggle_real_size();
                        }
                        show_grid(&grid, index.clone(), &window);
                    },
                    "z" => {
                        index.set(0);
                        show_grid(&grid, index.clone(), &window);
                    }
                    "n" => {
                        index.next();
                        show_grid(&grid, index.clone(), &window);
                    }
                    "p" => {
                        index.prev();
                        show_grid(&grid, index.clone(), &window);
                    }
                    "q" => {
                        index.save_marked_file_lists();
                        window.close();
                    },
                    "r" => {
                        index.random();
                        show_grid(&grid, index.clone(), &window);
                    },
                    "s" => {
                        index.toggle_to_select_current();
                        show_grid(&grid, index.clone(), &window);
                    },
                    "t" => {
                        index.toggle_to_touch_current();
                        show_grid(&grid, index.clone(), &window);
                    },
                    "u" => { 
                        index.toggle_to_unlink_current();
                        show_grid(&grid, index.clone(), &window);
                    },
                    "a" => {
                        index.start_area();
                    },
                    "e" => {
                        if index.current >= index.start_index {
                            for i in index.start_index .. index.current+1 {
                                index.toggle_to_select(i);
                            }
                        } else {
                            println!("area start index {} is greater than area end index {}", index.start_index, index.current);
                        }
                    },
                    "space" => { 
                        if let Some(_) = args.ordered { 
                            index.next()
                        } else {
                            index.random()
                        }
                        show_grid(&grid, index.clone(), &window);
                    },
                    "Right" => {
                        let h_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.hadjustment()))
                            .expect("Failed to get hadjustment");
                        h_adj.set_value(h_adj.value() + step as f64);
                    },
                    "Left" => {
                        let h_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.hadjustment()))
                            .expect("Failed to get hadjustment");
                        h_adj.set_value(h_adj.value() - step as f64);
                    },
                    "Down" => {
                        // Scroll down
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() + step as f64);
                    },
                    "Up" => {
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() - step as f64);
                    }
                    _ => { },
                };
                gtk::Inhibit(false)
            }
            else {
                gtk::Inhibit(false)
            }
        }));

        window.add_controller(evk);
        // show the first file
        if let Some(_) = args.ordered {
            let index: RefMut<'_,Index> = index_rc.borrow_mut();
            show_grid(&grid, index.clone(), &window);
        } else {
            let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
            index.random();
            show_grid(&grid, index.clone(), &window);
        }

        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong entries_rc, @strong grid, @strong index_rc, @strong window => move | | { 
                let mut index: RefMut<'_,Index> = index_rc.borrow_mut();
                if let Some(_) = args.ordered { 
                    index.next();
                } else {
                    index.random();
                };
                show_grid(&grid, index.clone(), &window);
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

fn show_grid(grid: &Grid, index: Index, window: &gtk::ApplicationWindow) {
    let entries = index.entries.clone();
    let selection_size = index.clone().selection_size();
    for i in 0 .. selection_size {
        let row = (i / index.grid_size) as i32;
        let col = (i % index.grid_size) as i32;
        let picture = grid.child_at(col,row).unwrap().downcast::<gtk::Picture>().unwrap();
        let j = index.clone().nth_index((row as usize) * index.grid_size + (col as usize));
        if entries[j].to_select {
            picture.set_opacity(0.25)
        } else {
            picture.set_opacity(1.0)
        }
        let filename = index.clone().nth_filename(i);
        // let current_index = index.current
        picture.set_can_shrink(!index.real_size);
        picture.set_filename(Some(filename.clone()));

    }
    window.set_title(Some(&format!("{} {} {} [{}] {} {}",
                index.current,
                &entries[index.current].file_path.as_str(),
                show_marks(&entries[index.current]),
                index.register,
                if index.real_size { "*" } else { ""},
                &entries[index.current].file_size)));
}
