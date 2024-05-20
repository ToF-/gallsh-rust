use clap::Parser;
use clap_num::number_range;
use entries::{Entries, update_thumbnails};
use image::get_image_color_size;
use entry::{Entry, EntryList, THUMB_SUFFIX, make_entry};
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::EventControllerMotion;
use gtk::prelude::*;
use gtk::traits::WidgetExt;
use gtk::{self, Application, ScrolledWindow, gdk, glib, Grid, Picture};
use order::{Order};
use std::cell::{Ref, RefCell, RefMut};
use std::env;
use std::rc::Rc;
use std::time::{Duration};
const FIRST_CELL: usize = 0;
const DEFAULT_WIDTH: i32 = 1000;



mod entry;
mod order;
mod entries;
mod image;


fn less_than_11(s: &str) -> Result<usize, String> {
    number_range(s,1,10)
}

// declarative setting of arguments
/// Gallery Show
#[derive(Parser, Clone, Debug)]
#[command(infer_subcommands = true, infer_long_args = true, author, version, about, long_about = None)]
/// Pattern that displayed files must have
struct Args {

    /// Pattern (only files with names matching the regular expression will be displayed)
    #[arg(short, long)]
    pattern: Option<String>,

    /// Maximized window
    #[arg(short, long, default_value_t = false, help("show the images in full screen"))]
    maximized: bool,

    /// Ordered display (or random)
    #[arg(short, long,value_name("order"), ignore_case(true), default_value_t = Order::Random)]
    order: Order,

    /// Date ordered display
    #[arg(short, long, default_value_t = false)]
    date: bool,

    /// Name ordered display
    #[arg(short, long, default_value_t = false)]
    name: bool,

    /// Size ordered display
    #[arg(short, long, default_value_t = false)]
    size: bool,

    /// Color size ordered display
    #[arg(short, long, default_value_t = false)]
    color: bool,

    /// Timer delay for next picture
    #[arg(long)]
    timer: Option<u64>,

    /// Directory to search (default is set with variable GALLSHDIR)
    directory: Option<String>,

    /// Reading List (only files in the list are displayed)
    #[arg(short, long)]
    reading: Option<String>,

    /// Index of first image to read 
    #[arg(short, long)]
    index: Option<usize>,

    /// Grid Size 
    #[arg(short, long, value_parser=less_than_11)]
    grid: Option<usize>,

    /// Low Limit on file size
    #[arg(long)]
    low: Option<u64>,

    /// High Limit on file size
    #[arg(long)]
    high: Option<u64>,

    /// From index number
    #[arg(long)]
    from: Option<usize>,

    /// To index number
    #[arg(long)]
    to: Option<usize>,

    /// File to view
    #[arg(short, long)]
    file: Option<String>, 

    /// Thumbnails only
    #[arg(short,long)]
    thumbnails: bool,

    /// Update thumbnails and then quit
    #[arg(short,long)]
    update_thumbnails: bool,

    /// Window width (default is set with GALLSHWIDTH)
    #[arg(short, long)]
    width: Option<i32>,
}

const DEFAULT_DIR :&str  = "images/";
const DIR_ENV_VAR :&str = "GALLSHDIR";
const WIDTH_ENV_VAR :&str = "GALLSHWIDTH";

fn main() {

    let args = Args::parse();
    let gallshdir = env::var(DIR_ENV_VAR);

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

    // clone! passes a strong reference to a variable in the closure that activates the application
    // move converts any variables captured by reference or mutable reference to variables captured by value.
    application.connect_activate(clone!(@strong args => move |application: &gtk::Application| { 
        let path = if let Some(directory_arg) = &args.directory {
            String::from(directory_arg)
        } else if let Ok(standard_dir) = &gallshdir {
            String::from(standard_dir)
        } else {
            println!("GALLSHDIR variable not set. Using {} as default.", DEFAULT_DIR);
            String::from(DEFAULT_DIR)
        };
        let candidate_width = match args.width {
            Some(n) => n,
            None => match env::var(WIDTH_ENV_VAR) {
                Ok(s) => match s.parse::<i32>() {
                    Ok(n) => n,
                    _ => {
                        println!("illegal width value, setting to default");
                        DEFAULT_WIDTH
                    }
                },
                _ => {
                    DEFAULT_WIDTH
                }
            }
        };
        let width = if candidate_width < 3000 && candidate_width > 100 {
            candidate_width
        } else {
            println!("illegal width value, setting to default");
            DEFAULT_WIDTH
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
            if let Ok((created, deleted)) = update_thumbnails(&path) {
                println!("{created} thumbnails added, {deleted} thumbnails deleted");
            }
            std::process::exit(0);
        }
        let order = if args.name {
            Order::Name
        } else if args.date {
            Order::Date
        } else if args.size {
            Order::Size
        } else if args.color {
            Order::ColorSize
        } else {
            args.order
        };

        let mut entries = if let Some(list_file_name) = &reading_list {
            match Entries::from_list(list_file_name, order, grid_size) {
                Ok(result) => result,
                _ => std::process::exit(1),
            }
        } else if let Some(file_name) = &args.file {
            match Entries::from_file(file_name, grid_size) {
                Ok(result) => result,
                _ => std::process::exit(1),
            }
        } else {
            let mut entries = match Entries::from_directory(&path, args.thumbnails, &args.pattern, args.low, args.high, args.from, args.to, order, grid_size) {
                Ok(result) => result,
                _ => std::process::exit(1),
            };
            entries.set_selected_images();
            entries
        };
        println!("{} files selected", entries.entry_list.len());
        if entries.clone().len() == 0 {
            application.quit();
            return
        }

        if let Some(index_number) = args.index {
            entries.jump(index_number);
        }
        let entries_rc = Rc::new(RefCell::new(entries));
        let offset_rc = Rc::new(RefCell::new(0));

        let height = width;
        // build the main window
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .default_width(width)
            .default_height(height)
            .build();

        let grid_scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .name("grid")
            .build();

        let view_scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .name("view")
            .build();

        let view = Grid::new();
        view.set_row_homogeneous(true);
        view.set_column_homogeneous(true);
        view.set_hexpand(true);
        view.set_vexpand(true);
        let stack = gtk::Stack::new();
        let image_view = Picture::new();
        let view_gesture = gtk::GestureClick::new();
        view_gesture.set_button(0);
        view_gesture.connect_pressed(clone!(@strong entries_rc, @strong stack, @strong grid_scrolled_window, @strong window => move |_,_, _, _| {
            stack.set_visible_child(&grid_scrolled_window);
        }));
        image_view.add_controller(view_gesture);
        view.attach(&image_view, 0, 0, 1, 1);
        view_scrolled_window.set_child(Some(&view));

        let grid = Grid::new();
        let _ = stack.add_child(&grid_scrolled_window);
        let _ = stack.add_child(&view_scrolled_window);
        window.set_child(Some(&stack));
        stack.set_visible_child(&view_scrolled_window);
        stack.set_visible_child(&grid_scrolled_window);
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(true);
        grid.set_hexpand(true);
        grid.set_vexpand(true);
        for row in 0 .. grid_size {
            for col in 0 .. grid_size {
                let image = Picture::new();
                grid.attach(&image, row as i32, col as i32, 1, 1);

                let select_gesture = gtk::GestureClick::new();
                select_gesture.set_button(1);
                select_gesture.connect_pressed(clone!(@strong entries_rc, @strong grid, @strong window => move |_,_, _, _| {
                    let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                    let offset = col * grid_size + row;
                    entries.toggle_select_area(offset);
                    show_grid(&grid, &entries.clone());
                    window.set_title(Some(&entries.clone().show_status(offset)));
                }));
                image.add_controller(select_gesture);

                let view_gesture = gtk::GestureClick::new();
                view_gesture.set_button(3);

                view_gesture.connect_pressed(clone!(@strong entries_rc, @strong grid, @strong view, @strong stack, @strong view_scrolled_window, @strong grid_scrolled_window, @strong window => move |_,_, _, _| {
                    let entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                    if entries.max_cells == 1 { return };
                    let offset = col * grid_size + row;
                    stack.set_visible_child(&view_scrolled_window);
                    show_view(&view, &entries, offset);
                    window.set_title(Some(&entries.clone().show_status(offset)));
                }));
                image.add_controller(view_gesture);

                let motion_controller = EventControllerMotion::new(); 
                motion_controller.connect_enter(clone!(@strong entries_rc, @strong offset_rc, @strong window => move |_,_,_| {
                    if let Ok(entries) = entries_rc.try_borrow() {
                        let mut offset: RefMut<'_,usize> = offset_rc.borrow_mut();
                        *offset = col * grid_size + row;
                        window.set_title(Some(&entries.clone().show_status(*offset)));
                    } else {
                    }
                }));

                image.add_controller(motion_controller)
            }
        }
        grid_scrolled_window.set_child(Some(&grid));

        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(clone!(@strong entries_rc, @strong offset_rc, @strong grid, @strong window => move |_, key, _, _| {
            let step = 100;
            let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
            if let Some(s) = key.name() {
                if stack.visible_child().unwrap() == view_scrolled_window {
                    stack.set_visible_child(&grid_scrolled_window);
                    return gtk::Inhibit(false)
                };
                match s.as_str() {
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let digit:usize = s.parse().unwrap();
                        entries.add_digit_to_resiter(digit);
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "BackSpace" => {
                        entries.remove_digit_to_register();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "g" => {
                        entries.go_to_register();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "j" => {
                        for _ in 0..10 {
                            entries.next()
                        }
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "b" => {
                        for _ in 0..10 {
                            entries.prev()
                        }
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "f" => {
                        if (entries.clone().max_cells) == 1 {
                            entries.toggle_real_size();
                            show_grid(&grid, &entries.clone());
                            window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                        }
                    },
                    "z" => {
                        entries.jump(0);
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    }
                    "n"|"e" => {
                        entries.next();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    }
                    "p"|"i" => {
                        entries.prev();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    }
                    "q"|"Escape" => {
                        entries.save_marked_file_lists(args.thumbnails);
                        window.close();
                    },
                    "r" => {
                        entries.random();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "comma" => {
                        let offset = if args.thumbnails {
                            let offset: Ref<'_, usize> = offset_rc.borrow();
                            *offset
                        } else {
                            FIRST_CELL
                        };
                        entries.toggle_select(offset);
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "Return" => {
                        let offset = if args.thumbnails {
                            let offset: Ref<'_, usize> = offset_rc.borrow();
                            *offset
                        } else {
                            FIRST_CELL
                        };
                        entries.toggle_select_area(offset);
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    }
                    "a" => {
                        entries.set_grid_select();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "u" => {
                        entries.reset_grid_select();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "U" => {
                        entries.reset_all_select();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                    },
                    "period"|"k" => {
                        if stack.visible_child().unwrap() == grid_scrolled_window {
                            let offset: Ref<'_,usize> = offset_rc.borrow();
                            stack.set_visible_child(&view_scrolled_window);
                            show_view(&view, &entries, *offset);

                        } else {
                            stack.set_visible_child(&grid_scrolled_window);
                        }
                    },
                    "space" => { 
                        entries.next();
                        show_grid(&grid, &entries.clone());
                        window.set_title(Some(&(entries.clone().show_status(FIRST_CELL))));
                    },
                    "Right" => {
                        if entries.real_size { 
                            let h_adj = window
                                .child()
                                .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                .and_then(|sw| Some(sw.hadjustment()))
                                .expect("Failed to get hadjustment");
                            h_adj.set_value(h_adj.value() + step as f64);
                        } else {
                            entries.next();
                            show_grid(&grid, &entries.clone());
                            window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                        }
                    },
                    "Left" => {
                        if entries.real_size { 
                            let h_adj = window
                                .child()
                                .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                .and_then(|sw| Some(sw.hadjustment()))
                                .expect("Failed to get hadjustment");
                            h_adj.set_value(h_adj.value() - step as f64);
                        } else {
                            entries.prev();
                            show_grid(&grid, &entries.clone());
                            window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                        }
                    },
                    "Down" => {
                        // Scroll down
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() + step as f64);
                    },
                    "Up" => {
                        let v_adj = window
                            .child()
                            .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                            .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                            .and_then(|sw| Some(sw.vadjustment()))
                            .expect("Failed to get vadjustment");
                        v_adj.set_value(v_adj.value() - step as f64);
                    },
                    s => { println!("{} ?", s) },
                };
                gtk::Inhibit(false)
            }
            else {
                gtk::Inhibit(false)
            };
            gtk::Inhibit(false)
    }));

    window.add_controller(evk);
    // show the first file
        let entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
        show_grid(&grid, &entries);
        window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong entries_rc, @strong grid, @strong window => move | | { 
                let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                entries.next();
                show_grid(&grid, &entries.clone());
                window.set_title(Some(&entries.clone().show_status(FIRST_CELL)));
                Continue(true) 
            }));
        };
        window.present();
    }));
    application.set_accels_for_action("win.save", &["s"]);
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn show_grid(grid: &Grid, entries: &Entries) {
    let max_cells = entries.clone().max_cells;
    let side = match max_cells { 4 => 2, 9 => 3, 16 => 4, 25 => 5, 36 => 6, 49 => 7, 64 => 8, 81 => 9, 100 => 10, _ => 1, };
    for cell_index in 0 .. max_cells {
        let row = (cell_index / side) as i32;
        let col = (cell_index % side) as i32;
        let picture = grid.child_at(col,row).unwrap().downcast::<gtk::Picture>().unwrap();
        let offset = row as usize * side + col as usize;
        let entry = entries.clone().offset_entry(offset);
        let opacity = if entry.to_select { 0.50 } else { 1.0 };
        picture.set_opacity(opacity);
        let filename = entry.file_path;
        picture.set_can_shrink(!entries.clone().real_size);
        picture.set_filename(Some(filename.clone()));
        picture.set_visible(entries.current + offset <= entries.maximum);
    }
}

fn show_view(grid: &Grid, entries: &Entries, offset: usize) {
    let entry = entries.clone().offset_entry(offset);
    let file_path = entry.original_file_path();
    let picture = grid.child_at(0,0).unwrap().downcast::<gtk::Picture>().unwrap();
    picture.set_filename(Some(file_path));
}
