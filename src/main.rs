use clap::Parser;
use clap_num::number_range;
use entries::{Entries, update_thumbnails};
use entry::{Entry, EntryList, THUMB_SUFFIX, make_entry};
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::EventControllerMotion;
use gtk::prelude::*;
use gtk::traits::WidgetExt;
use gtk::{self, Application, ScrolledWindow, gdk, glib, Grid, Picture};
use order::{Order};
use rank::{Rank};
use std::cell::{RefCell, RefMut};
use std::env;
use std::rc::Rc;
use std::time::{Duration};
const DEFAULT_WIDTH: i32 = 1000;
const DEFAULT_HEIGHT: i32 = 1000;



mod entry;
mod order;
mod entries;
mod image;
mod rank;


fn less_than_11(s: &str) -> Result<usize, String> {
    number_range(s,1,10)
}

// declarative setting of arguments
/// Gallery Show
#[derive(Parser, Clone, Debug)]
#[command(infer_subcommands = true, infer_long_args = true, author, version, about, long_about = None)]
/// Pattern that displayed files must have
struct Args {

    /// Directory to search (default is set with variable GALLSHDIR)
    directory: Option<String>,

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

    /// Rank value ordered display
    #[arg(short, long, default_value_t = false)]
    value:bool,

    /// Size ordered display
    #[arg(short, long, default_value_t = false)]
    size: bool,

    /// Colors size ordered display
    #[arg(short, long, default_value_t = false)]
    colors: bool,

    /// Timer delay for next picture
    #[arg(long)]
    timer: Option<u64>,

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

    /// Move selection to a target folder
    #[arg(long)]
    copy_selection: Option<String>,

    /// Window width (default is set with GALLSHWIDTH)
    #[arg(short, long)]
    width: Option<i32>,
    ///
    /// Window width (default is set with GALLSHHEIGHT)
    #[arg(short, long)]
    height: Option<i32>,
}

const DEFAULT_DIR :&str  = "images/";
const DIR_ENV_VAR :&str = "GALLSHDIR";
const WIDTH_ENV_VAR :&str = "GALLSHWIDTH";
const HEIGHT_ENV_VAR :&str = "GALLSHHEIGHT";

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
        let candidate_height = match args.height {
            Some(n) => n,
            None => match env::var(HEIGHT_ENV_VAR) {
                Ok(s) => match s.parse::<i32>() {
                    Ok(n) => n,
                    _ => {
                        println!("illegal height value, setting to default");
                        DEFAULT_HEIGHT
                    }
                },
                _ => {
                    DEFAULT_HEIGHT
                }
            }
        };
        let height = if candidate_height < 3000 && candidate_height > 100 {
            candidate_height
        } else {
            println!("illegal height value, setting to default");
            DEFAULT_HEIGHT
        };
        let reading_list = &args.reading;
        let copy_selection_target: Option<String> = match &args.copy_selection {
            Some(target) => Some(target.to_string()),
            None => None,
        };

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
        } else if args.colors {
            Order::Colors
        } else if args.value {
            Order::Value
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
        if entries.len() == 0 {
            application.quit();
            return
        }

        if let Some(index_number) = args.index {
            entries.jump(index_number);
        }
        let entries_rc = Rc::new(RefCell::new(entries));

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
                    entries.offset = offset;
                    entries.toggle_select_area();
                    show_grid(&grid, &entries, &window);
                }));
                image.add_controller(select_gesture);

                let view_gesture = gtk::GestureClick::new();
                view_gesture.set_button(3);

                view_gesture.connect_pressed(clone!(@strong entries_rc, @strong grid, @strong view, @strong stack, @strong view_scrolled_window, @strong grid_scrolled_window, @strong window => move |_,_, _, _| {
                    let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                    if entries.max_cells == 1 { return };
                    let offset = col * grid_size + row;
                    entries.offset = offset;
                    stack.set_visible_child(&view_scrolled_window);
                    show_view(&view, &entries, &window);
                }));
                image.add_controller(view_gesture);

                let motion_controller = EventControllerMotion::new(); 
                motion_controller.connect_enter(clone!(@strong entries_rc, @strong window => move |_,_,_| {
                    // let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                    if let Ok(mut entries) = entries_rc.try_borrow_mut() {
                        let offset = col * grid_size + row;
                        entries.offset = offset;
                        window.set_title(Some(&(entries.status())));
                    } else { }
                }));

                image.add_controller(motion_controller)
            }
        }
        grid_scrolled_window.set_child(Some(&grid));

        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(clone!(@strong entries_rc, @strong grid, @strong window => move |_, key, _, _| {
            let step = 100;
            if let Ok(mut entries) = entries_rc.try_borrow_mut() {
                if let Some(s) = key.name() {
                    if stack.visible_child().unwrap() == view_scrolled_window {
                        stack.set_visible_child(&grid_scrolled_window);
                        return gtk::Inhibit(false)
                    };
                    let mut show = true;
                    match s.as_str() {
                        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                            let digit:usize = s.parse().unwrap();
                            entries.add_digit_to_register(digit);
                        },
                        "BackSpace" => entries.remove_digit_to_register(),
                        "g" => if ! entries.register.is_none() { entries.go_to_register() },
                        "j" => for _ in 0..10 { entries.next() },
                        "l" => for _ in 0..10 { entries.prev() },
                        "f" => if (entries.max_cells) == 1 { entries.toggle_real_size() },
                        "z" => entries.jump(0),
                        "n"|"e" => entries.next(),
                        "p"|"i" => entries.prev(),
                        "q"|"Escape" => {
                            entries.save_marked_file_lists(args.thumbnails);
                            entries.save_updated_ranks();
                            window.close();
                        },
                        "Q" => if let Some(target_path) = &copy_selection_target {
                                entries.copy_selection(&target_path);
                                entries.save_marked_file_lists(args.thumbnails);
                                entries.save_updated_ranks();
                                window.close()
                            },
                        "r" => entries.random(),
                        "M" => entries.toggle_rank_area(Rank::ONE_STAR),
                        "N" => entries.toggle_rank_area(Rank::TWO_STARS),
                        "O" => entries.toggle_rank_area(Rank::THREE_STARS),
                        "comma" => entries.toggle_select(),
                        "Return" => entries.toggle_select_area(),
                        "asterisk"|"A" => entries.set_rank(Rank::THREE_STARS),
                        "slash"|"B" => entries.set_rank(Rank::TWO_STARS),
                        "minus"|"C" => entries.set_rank(Rank::ONE_STAR),
                        "plus"|"D" => entries.set_rank(Rank::NO_STAR),
                        "at" => entries.unset_grid_ranks(),
                        "a" => entries.set_grid_select(),
                        "u" => entries.reset_grid_select(),
                        "U" => entries.reset_all_select(),
                        "period"|"k" => {
                            if stack.visible_child().unwrap() == grid_scrolled_window {
                                show_grid(&grid, &entries, &window);
                                stack.set_visible_child(&view_scrolled_window);
                                show_view(&view, &entries, &window);
                                show = false
                            } else {
                                stack.set_visible_child(&grid_scrolled_window)
                            }
                        },
                        "space" => entries.next(),
                        "Right" => if entries.real_size { 
                            show = false;
                            let h_adj = window
                                .child()
                                .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                .and_then(|sw| Some(sw.hadjustment()))
                                .expect("Failed to get hadjustment");
                            h_adj.set_value(h_adj.value() + step as f64)
                        } else {
                            entries.next()
                        },
                        "Left" => if entries.real_size { 
                                show = false;
                                let h_adj = window
                                    .child()
                                    .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                    .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                    .and_then(|sw| Some(sw.hadjustment()))
                                    .expect("Failed to get hadjustment");
                                h_adj.set_value(h_adj.value() - step as f64)
                            } else {
                                entries.prev()
                            },
                        "Down" => if entries.real_size {
                                show = false;
                                let v_adj = window
                                    .child()
                                    .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                    .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                    .and_then(|sw| Some(sw.vadjustment()))
                                    .expect("Failed to get vadjustment");
                                v_adj.set_value(v_adj.value() + step as f64)
                            },
                        "Up" => if entries.real_size {
                                show = false;
                                let v_adj = window
                                    .child()
                                    .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                    .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                    .and_then(|sw| Some(sw.vadjustment()))
                                    .expect("Failed to get vadjustment");
                                v_adj.set_value(v_adj.value() - step as f64)
                            },
                        s => { println!("{} ?", s) },
                    };
                    if show {
                        show_grid(&grid, &entries, &window);
                    }
                    gtk::Inhibit(false)
                }
                else {
                    gtk::Inhibit(false)
                }
            } else {
                gtk::Inhibit(false)
            }
        }));

        window.add_controller(evk);
        // show the first file
        let entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
        show_grid(&grid, &entries, &window);
        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong entries_rc, @strong grid, @strong window => move | | { 
                let mut entries: RefMut<'_,Entries> = entries_rc.borrow_mut();
                entries.next();
                show_grid(&grid, &entries, &window);
                window.set_title(Some(&entries.status()));
                Continue(true) 
            }));
        };
        window.present();
    }));
    application.set_accels_for_action("win.save", &["s"]);
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn show_grid(grid: &Grid, entries: &Entries, window: &gtk::ApplicationWindow) {
    let max_cells = entries.max_cells;
    let cells_per_row = entries.cells_per_row;
    let current = entries.current;
    let maximum = entries.maximum;
    for cell_index in 0 .. max_cells {
        let row = (cell_index / cells_per_row) as i32;
        let col = (cell_index % cells_per_row) as i32;
        let picture = grid.child_at(col,row).unwrap().downcast::<gtk::Picture>().unwrap();
        let offset = row as usize * cells_per_row + col as usize;
        let position = current + offset;
        if position <= maximum {
            let entry = &entries.entry_list[position];
            let opacity = if entry.to_select { 0.50 } else { 1.0 };
            picture.set_opacity(opacity);
            let filename = &entry.file_path;
            picture.set_can_shrink(!entries.real_size);
            picture.set_filename(Some(filename.to_string()));
            picture.set_visible(true)
        } else {
            picture.set_visible(false)
        }
    };
    window.set_title(Some(&entries.status()));
}

fn show_view(grid: &Grid, entries: &Entries, window: &gtk::ApplicationWindow) {
    let entry = entries.entry();
    let file_path = entry.original_file_path();
    let picture = grid.child_at(0,0).unwrap().downcast::<gtk::Picture>().unwrap();
    picture.set_filename(Some(file_path));
    window.set_title(Some(&entries.status()));
}
