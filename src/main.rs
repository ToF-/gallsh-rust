use crate::repository::Repository;
use crate::picture_io::{entries_from_reading_list, entries_from_directory, entries_from_file, set_original_picture_file, set_thumbnail_picture_file};
use clap::Parser;
use clap_num::number_range;
use entry::{Entry, EntryList, make_entry};
use paths::THUMB_SUFFIX; 
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::EventControllerMotion;
use gtk::prelude::*;
use gtk::traits::WidgetExt;
use gtk::{self, Align, Application, CssProvider, Orientation, Label, ScrolledWindow, gdk, glib, Grid, Picture};
use order::{Order};
use rank::{Rank};
use std::cell::{RefCell, RefMut};
use std::env;
use std::rc::Rc;
use std::time::{Duration};

const DEFAULT_WIDTH: i32 = 1000;
const DEFAULT_HEIGHT: i32 = 1000;


mod picture_io;
mod entry;
mod image;
mod navigator;
mod order;
mod paths;
mod rank;
mod repository;


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

        let entry_list = if let Some(list_file_name) = reading_list {
            match entries_from_reading_list(list_file_name, args.pattern.clone()) {
                Ok(list) => list,
                _ => {
                    application.quit();
                    return
                },
            }
        } else if let Some(file_name) = &args.file {
            match entries_from_file(file_name) {
                Ok(list) => list,
                _ => {
                    application.quit();
                    return
                },
            }
        } else {
            match entries_from_directory(&path, args.pattern.clone()) {
                Ok(list) => list,
                _ => {
                    application.quit();
                    return
                },
            }
        };

        let mut repository = Repository::from_entries(entry_list, grid_size);
        repository.sort_by(order);
        repository.slice(args.from, args.to);
        repository.read_select_entries();

        println!("{} entries", repository.navigator.capacity());
        if repository.navigator.capacity() == 0 {
            application.quit();
            return
        };

        if let Some(index) = args.index {
            if repository.navigator.can_move_to_index(index) {
                repository.navigator.move_to_index(index)
            }
        };
        let repository_rc = Rc::new(RefCell::new(repository));

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

        let buttons_css_provider = CssProvider::new();
        buttons_css_provider.load_from_data(
            "
            label {
                color: gray;
                font-size: 12px;
            }
            text-button {
                background-color: black;
            }
        ");
        let view = Grid::new();
        view.set_row_homogeneous(true);
        view.set_column_homogeneous(true);
        view.set_hexpand(true);
        view.set_vexpand(true);
        let stack = gtk::Stack::new();
        let image_view = Picture::new();
        let view_gesture = gtk::GestureClick::new();
        view_gesture.set_button(0);
        view_gesture.connect_pressed(clone!(@strong repository_rc, @strong stack, @strong grid_scrolled_window, @strong window => move |_,_, _, _| {
            stack.set_visible_child(&grid_scrolled_window);
        }));
        image_view.add_controller(view_gesture);
        view.attach(&image_view, 0, 0, 1, 1);
        view_scrolled_window.set_child(Some(&view));

        let panel = Grid::new();
        panel.set_hexpand(true);
        panel.set_vexpand(true);
        panel.set_row_homogeneous(true);
        panel.set_column_homogeneous(false);
        let left_button = Label::new(Some("←"));
        let right_button = Label::new(Some("→"));
        left_button.set_width_chars(10);
        right_button.set_width_chars(10);
        left_button.style_context().add_provider(&buttons_css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        right_button.style_context().add_provider(&buttons_css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        let left_gesture = gtk::GestureClick::new();

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
        panel.attach(&left_button, 0, 0, 1, 1);
        panel.attach(&grid, 1, 0, 1, 1);
        panel.attach(&right_button, 2, 0, 1, 1);
        left_gesture.set_button(1);
        left_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong window => move |_,_,_,_| {
            let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
            repository.navigator.move_prev_page();
            show_grid(&grid, &repository, &window);
        }));
        left_button.add_controller(left_gesture);
        let right_gesture = gtk::GestureClick::new();
        right_gesture.set_button(1);
        right_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong window => move |_,_,_,_| {
            let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
            repository.navigator.move_next_page();
            show_grid(&grid, &repository, &window);
        }));
        right_button.add_controller(right_gesture);
        for col in 0 .. grid_size as i32 {
            for row in 0 .. grid_size as i32 {
                let vbox = gtk::Box::new(Orientation::Vertical, 0);
                let image = Picture::new();
                let label = Label::new(None);
                let style_context = label.style_context();
                style_context.add_provider(&buttons_css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
                vbox.set_valign(Align::Center);
                vbox.set_halign(Align::Center);
                vbox.append(&image);
                vbox.append(&label);
                grid.attach(&vbox, col as i32, row as i32, 1, 1);

                let select_gesture = gtk::GestureClick::new();
                select_gesture.set_button(1);
                select_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong window => move |_,_, _, _| {
                    let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                    if repository.navigator.can_move_abs((col,row)) {
                        repository.navigator.move_abs((col,row));
                        repository.select_point();
                    }
                    show_grid(&grid, &repository, &window);
                }));
                image.add_controller(select_gesture);

                let view_gesture = gtk::GestureClick::new();
                view_gesture.set_button(3);

                view_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong image, @strong view, @strong stack, @strong view_scrolled_window, @strong grid_scrolled_window, @strong window => move |_, _, _, _| {
                    let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                    if repository.navigator.cells_per_row() == 1 { return };
                    if repository.navigator.can_move_abs((col,row)) {
                        repository.navigator.move_abs((col,row));
                        repository.select_point();
                        stack.set_visible_child(&view_scrolled_window);
                        show_view(&view, &repository, &window);
                    }
                }));
                image.add_controller(view_gesture);

                let motion_controller = EventControllerMotion::new();
                motion_controller.connect_enter(clone!(@strong repository_rc, @strong grid, @strong label, @strong window => move |_,_,_| {
                    if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                        if repository.navigator.can_move_abs((col,row)) {
                            repository.navigator.move_abs((col,row));
                            if let Some(entry) = repository.current_entry() {
                                label.set_text(&entry.label_display(true))
                            };
                            window.set_title(Some(&(repository.title_display())));
                        } else {
                            println!("{:?} refused {:?}", (col,row), repository.navigator)
                        }
                    }
                }));

                motion_controller.connect_leave(clone!(@strong repository_rc, @strong grid, @strong label, @strong window => move |_| {
                    if let Ok(repository) = repository_rc.try_borrow_mut() {
                        if let Some(index) = repository.navigator.index_from_position((col, row)) {
                            let entry: &Entry = &repository.entry_list[index];
                            label.set_text(&entry.label_display(false));
                        }
                    };
                }));
                image.add_controller(motion_controller)
            }
        }
        grid_scrolled_window.set_child(Some(&panel));

        let evk = gtk::EventControllerKey::new();
        evk.connect_key_pressed(clone!(@strong repository_rc, @strong grid, @strong window => move |_, key, _, _| {
            let step = 100;
            if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                if let Some(s) = key.name() {
                    if stack.visible_child().unwrap() == view_scrolled_window {
                        stack.set_visible_child(&grid_scrolled_window);
                        return gtk::Inhibit(false)
                    };
                    let mut show = true;
                    match s.as_str() {
                        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                            let digit:usize = s.parse().unwrap();
                            repository.add_register_digit(digit);
                            println!("register index: {}", repository.register.unwrap())
                        },
                        "BackSpace" => {
                            repository.delete_register_digit();
                            match repository.register {
                                Some(index) => println!("register index: {}", index),
                                None => {},
                            }
                        },
                        "Return" => repository.select_point(),
                        "comma" => repository.point_select(),
                        "Escape" => repository.cancel_point(),

                        "g" => {
                            match repository.register {
                                Some(index) => println!("go to register index: {}", index),
                                None => println!("no register index"),
                            };
                            repository.move_to_register()
                        },
                        "j" => {
                            for _ in 0..10 { repository.navigator.move_next_page() };
                            println!("move forward ten pages")
                        },
                        "l" => {
                            for _ in 0..10 { repository.navigator.move_prev_page() };
                            println!("move backward ten pages")
                        },
                        "f" => if repository.navigator.cells_per_row() == 1 { 
                            repository.toggle_real_size();
                            println!("toggle real size")
                        },
                        "z" => {
                            repository.navigator.move_to_index(0);
                            println!("move to index 0")
                        },
                        "e" => {
                            repository.navigator.move_next_page();
                            println!("move to next page")
                        },
                        "n" => {
                            if repository.order.is_none() {
                                repository.sort_by(Order::Name);
                                println!("sort pictures by name");
                                show_grid(&grid, &repository, &window)
                            } else {
                                repository.navigator.move_next_page();
                                println!("move to next page")
                            }
                        },
                        "p"|"i" => {
                            repository.navigator.move_prev_page();
                            println!("move to prev page")
                        },
                        "q" => {
                            repository.save_updated_ranks();
                            repository.save_select_entries();
                            println!("quit gallery show");
                            window.close();
                        },
                        "Q" => {
                            repository.save_updated_ranks();
                            repository.save_select_entries();
                            if let Some(target_path) = &copy_selection_target {
                                println!("copy selection to target path");
                                repository.copy_select_entries(&target_path)
                            };
                            println!("quit gallery show");
                            window.close()
                        },
                        "B"|"plus"|"D" => repository.point_rank(Rank::NoStar),
                        "M"|"Eacute"|"minus"|"C" => repository.point_rank(Rank::OneStar),
                        "N"|"P"|"slash" => repository.point_rank(Rank::TwoStars),
                        "asterisk"|"A"|"O" => repository.point_rank(Rank::ThreeStars),
                        "c" => if repository.order.is_none() {
                            repository.sort_by(Order::Colors);
                            show_grid(&grid, &repository, &window)
                        },
                        "d" => if repository.order.is_none() {
                            repository.sort_by(Order::Date);
                            show_grid(&grid, &repository, &window)
                        },
                        "R" => repository.set_rank(Rank::NoStar),
                        "r" => {
                            if repository.order.is_none() {
                                repository.sort_by(Order::Random);
                                show_grid(&grid, &repository, &window)
                            } else {
                                repository.navigator.move_to_random_index()
                            }
                        },
                        "a" => repository.select_page(true),
                        "u" => repository.select_page(false),
                        "U" => repository.select_all(false),
                        "s" => { 
                            if repository.order.is_none() {
                                repository.sort_by(Order::Size);
                                show_grid(&grid, &repository, &window)
                            } else {
                                repository.select_point()
                            }
                        },
                        "o" => repository.order = None,
                        "v" => if repository.order.is_none() {
                            repository.sort_by(Order::Value);
                            show_grid(&grid, &repository, &window)
                        },
                        "period"|"k" => {
                            if stack.visible_child().unwrap() == grid_scrolled_window {
                                show_grid(&grid, &repository, &window);
                                stack.set_visible_child(&view_scrolled_window);
                                show_view(&view, &repository, &window);
                                show = false
                            } else {
                                stack.set_visible_child(&grid_scrolled_window)
                            }
                        },
                        "space" => repository.navigator.move_next_page(),
                        "Right" => {
                            show = false;
                            if repository.real_size() {
                                let h_adj = window
                                    .child()
                                    .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                    .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                    .and_then(|sw| Some(sw.hadjustment()))
                                    .expect("Failed to get hadjustment");
                                h_adj.set_value(h_adj.value() + step as f64)
                            } else {
                                if repository.navigator.cells_per_row() == 1 {
                                    repository.navigator.move_next_page();
                                    show = true;
                                } else {
                                    navigate(&mut repository, &grid, &window, 1, 0);
                                }
                            }
                        },
                        "Left" => {
                            show = false;
                            if repository.real_size() {
                                let h_adj = window
                                    .child()
                                    .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                    .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                    .and_then(|sw| Some(sw.hadjustment()))
                                    .expect("Failed to get hadjustment");
                                h_adj.set_value(h_adj.value() - step as f64)
                            } else {
                                if repository.navigator.cells_per_row() == 1 {
                                    repository.navigator.move_prev_page();
                                    show = true;
                                } else {
                                    navigate(&mut repository, &grid, &window, -1, 0);
                                }
                            }
                        },
                        "Down" => {
                            show = false;
                            if repository.real_size() {
                                let v_adj = window
                                    .child()
                                    .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                    .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                    .and_then(|sw| Some(sw.vadjustment()))
                                    .expect("Failed to get vadjustment");
                                v_adj.set_value(v_adj.value() + step as f64)
                            } else {
                                if repository.navigator.cells_per_row() == 1 {
                                    repository.navigator.move_next_page()
                                } else {
                                    navigate(&mut repository, &grid, &window, 0, 1);
                                }
                            }
                        },
                        "Up" => {
                            show = false;
                            if repository.real_size() {
                                let v_adj = window
                                    .child()
                                    .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
                                    .and_then(|child| child.downcast::<ScrolledWindow>().ok())
                                    .and_then(|sw| Some(sw.vadjustment()))
                                    .expect("Failed to get vadjustment");
                                v_adj.set_value(v_adj.value() - step as f64)
                            } else {
                                if repository.navigator.cells_per_row() == 1 {
                                    repository.navigator.move_next_page();
                                } else {
                                    navigate(&mut repository, &grid, &window, 0, -1);
                                }
                            }
                        },
                        _ => {},
                    };
                    if show {
                        show_grid(&grid, &repository, &window);
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
        let repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
        show_grid(&grid, &repository, &window);
        if args.maximized { window.fullscreen() };
        // if a timer has been passed, set a timeout routine
        if let Some(t) = args.timer {
            timeout_add_local(Duration::new(t,0), clone!(@strong repository_rc, @strong grid, @strong window => move | | {
                let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                repository.navigator.move_next_page();
                show_grid(&grid, &repository, &window);
                window.set_title(Some(&repository.title_display()));
                Continue(true)
            }));
        };
        window.present();
    }));
    application.set_accels_for_action("win.save", &["s"]);
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn show_grid(grid: &Grid, repository: &Repository, window: &gtk::ApplicationWindow) {
    let cells_per_row = repository.navigator.cells_per_row();
    for col in 0..cells_per_row {
        for row in 0..cells_per_row {
            let vbox = grid.child_at(col,row).unwrap().downcast::<gtk::Box>().unwrap();
            let picture = vbox.first_child().unwrap().downcast::<gtk::Picture>().unwrap();
            let label = vbox.last_child().unwrap().downcast::<gtk::Label>().unwrap();
            if let Some(index) = repository.navigator.index_from_position((col,row)) {
                let entry = &repository.entry_list[index];
                let status = format!("{} {} {}",
                    if index == repository.navigator.index() && cells_per_row > 1 { "▄" } else { "" },
                    entry.rank.show(),
                    if entry.to_select { "△" } else { "" });
                label.set_text(&status);
                let opacity = if entry.to_select { 0.50 } else { 1.0 };
                picture.set_opacity(opacity);
                picture.set_can_shrink(!repository.real_size());
                if repository.navigator.cells_per_row() < 10 {
                    match set_original_picture_file(&picture, &entry) {
                        Ok(_) => {
                            picture.set_visible(true)
                        },
                        Err(err) => {
                            picture.set_visible(false);
                            println!("{}",err.to_string())
                        },
                    }
                } else {
                    match set_thumbnail_picture_file(&picture, &entry) {
                        Ok(_) => {
                            picture.set_visible(true)
                        },
                        Err(err) => {
                            picture.set_visible(false);
                            println!("{}",err.to_string())
                        },
                    }
                };
            } else {
                picture.set_visible(false);
                label.set_text("");
            }
        }
    }
    window.set_title(Some(&repository.title_display()));
}

fn show_view(grid: &Grid, repository: &Repository, window: &gtk::ApplicationWindow) {
    let entry = repository.current_entry().unwrap();
    let picture = grid.first_child().unwrap().downcast::<gtk::Picture>().unwrap();
    match set_original_picture_file(&picture, &entry) {
        Ok(_) => {
            picture.set_visible(true);
            window.set_title(Some(&repository.title_display()))
        },
        Err(err) => {
            picture.set_visible(false);
            println!("{}",err.to_string())
        },
    }
}

fn label_at(grid: &gtk::Grid, col: i32, row: i32) -> gtk::Label {
    grid.child_at(col as i32, row as i32).unwrap()
        .downcast::<gtk::Box>().unwrap()
        .last_child().unwrap()
        .downcast::<gtk::Label>().unwrap()
}

fn navigate(repository: &mut Repository, grid: &gtk::Grid, window: &gtk::ApplicationWindow, col_move: i32, row_move: i32) {
    if repository.navigator.can_move_rel((col_move, row_move)) {
        let old_coords = repository.navigator.position();
        let old_label = label_at(&grid, old_coords.0, old_coords.1);
        let old_index = repository.navigator.index();
        old_label.set_text(&repository.entry_list[old_index].label_display(false));
        repository.navigator.move_rel((col_move, row_move));
        let new_coords = repository.navigator.position();
        let new_label = label_at(&grid, new_coords.0, new_coords.1);
        let new_index = repository.navigator.index();
        new_label.set_text(&repository.entry_list[new_index].label_display(true));
        window.set_title(Some(&repository.title_display()));
    }
}
