use clap::Parser;
use crate::args::Args;
use crate::direction::Direction;
use crate::navigator::Coords;
use crate::paths::determine_path;
use crate::picture_io::draw_palette;
use crate::picture_io::ensure_thumbnail;
use crate::picture_io::is_valid_path;
use crate::picture_io::{read_entries, set_original_picture_file, set_thumbnail_picture_file};
use crate::repository::Repository;
use entry::{Entry, EntryList, make_entry};
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::prelude::*;
use gtk::traits::WidgetExt;
use gtk::{self, Align, Application, CssProvider, Orientation, Label, ScrolledWindow, gdk, glib, Grid, Picture};
use order::{Order};
use paths::THUMB_SUFFIX;
use rank::{Rank};
use std::cell::{RefCell, RefMut};
use std::env;
use std::rc::Rc;
use std::time::{Duration};

const DEFAULT_WIDTH: i32   = 1000;
const DEFAULT_HEIGHT: i32  = 1000;
const WIDTH_ENV_VAR :&str  = "GALLSHWIDTH";
const HEIGHT_ENV_VAR :&str = "GALLSHHEIGHT";

pub struct Graphics {
    pub application_window:   gtk::ApplicationWindow,
    pub stack:                gtk::Stack,
    pub grid_scrolled_window: gtk::ScrolledWindow,
    pub view_scrolled_window: gtk::ScrolledWindow,
    pub picture_grid:       gtk::Grid,
    pub image_view:         gtk::Picture,
}

mod direction;
mod picture_io;
mod entry;
mod image;
mod image_data;
mod navigator;
mod order;
mod paths;
mod rank;
mod repository;
mod args;



fn main() {
    let args = Args::parse();

    let application = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    application.connect_startup(|_| {
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data("window { background-color:black;} image { margin:1em ; } label { color:white; }");
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css_provider,
            1000,
        );
    });

    // clone! passes a strong reference to a variable in the closure that activates the application
    // move converts any variables captured by reference or mutable reference to variables captured by value.
    application.connect_activate(clone!(@strong args => move |application: &gtk::Application| {
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
        let copy_selection_target: Option<String> = match &args.copy_selection {
            Some(target) => {
                if is_valid_path(target) {
                    Some(target.to_string())
                } else {
                    eprintln!("path {} doesn't exist", target);
                    application.quit();
                    None
                }
            },
            None => None,
        };

        let move_selection_target: Option<String> = match &args.move_selection {
            Some(target) => {
                if is_valid_path(target) {
                    Some(target.to_string())
                } else {
                    eprintln!("path {} doesn't exist", target);
                    application.quit();
                    None
                }
            },
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

        let order = Order::from_options(args.name, args.date, args.size, args.colors, args.value, args.palette, args.label);
        let path = determine_path(args.directory.clone());
        let entry_list = match read_entries(args.reading.clone(), args.file.clone(), path, args.pattern.clone()) {
            Ok(list) => list,
            Err(err) => {
                println!("{}", err);
                application.quit();
                return
            }
        };
        if args.update_image_data {
            for entry in &entry_list {
                let _ = ensure_thumbnail(&entry);
            };
            application.quit()
        };

        let mut repository = Repository::from_entries(entry_list, grid_size);
        repository.sort_by(order);
        repository.slice(args.from, args.to);

        println!("{} entries", repository.capacity());
        if repository.capacity() == 0 {
            application.quit();
            return
        };

        if let Some(index) = args.index {
            repository.move_to_index(index)
        };

        if args.extraction {
            repository.toggle_palette_extract();
        }
        let repository_rc = Rc::new(RefCell::new(repository));

        // build the main window
        // here's the deal:
        //
        //  window: ApplicationWindow
        //      stack: Stack
        //          grid_scrolled_window: ScrolledWindow
        //              panel: Grid
        //                  left_button: Label
        //                  grid: Grid
        //                      { cells_per_row x cells_per_row }
        //                      …
        //                      vbox: Box
        //                          image: Picture
        //                          label: Label
        //                      …
        //                  right_button: Label
        //          view_scrolled_window: ScrolledWindow
        //              view: Grid
        //                  image_view: Picture
        //

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
        view_scrolled_window.set_child(Some(&view));

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        let _ = stack.add_child(&grid_scrolled_window);
        let _ = stack.add_child(&view_scrolled_window);
        stack.set_visible_child(&view_scrolled_window);
        stack.set_visible_child(&grid_scrolled_window);

        window.set_child(Some(&stack));

        let image_view = Picture::new();
        let view_gesture = gtk::GestureClick::new();
        view_gesture.set_button(0);
        view_gesture.connect_pressed(clone!(@strong repository_rc, @strong stack, @strong grid_scrolled_window, @strong window => move |_,_, _, _| {
            stack.set_visible_child(&grid_scrolled_window);
        }));

        image_view.add_controller(view_gesture);

        view.attach(&image_view, 0, 0, 1, 1);


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

        let picture_grid = Grid::new();
        picture_grid.set_widget_name("picture_grid");
        picture_grid.set_row_homogeneous(true);
        picture_grid.set_column_homogeneous(true);
        picture_grid.set_hexpand(true);
        picture_grid.set_vexpand(true);
        if grid_size > 1 {
            panel.attach(&left_button, 0, 0, 1, 1);
            panel.attach(&picture_grid, 1, 0, 1, 1);
            panel.attach(&right_button, 2, 0, 1, 1);
        } else {
            panel.attach(&picture_grid, 0, 0, 1, 1);
        }
        left_gesture.set_button(1);
        left_gesture.connect_pressed(clone!(@strong repository_rc, @strong picture_grid, @strong window => move |_,_,_,_| {
            {
                let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                repository.move_prev_page();
            }
            setup_picture_grid(&repository_rc, &window);
        }));
        left_button.add_controller(left_gesture);
        let right_gesture = gtk::GestureClick::new();
        right_gesture.set_button(1);
        right_gesture.connect_pressed(clone!(@strong repository_rc, @strong picture_grid, @strong window => move |_,_,_,_| {
            {
                let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                repository.move_next_page();
            }
            setup_picture_grid(&repository_rc, &window);
        }));
        right_button.add_controller(right_gesture);
        for col in 0 .. grid_size as i32 {
            for row in 0 .. grid_size as i32 {
                let coords: Coords = (col,row);
                let vbox = gtk::Box::new(Orientation::Vertical, 0);
                vbox.set_valign(Align::Center);
                vbox.set_halign(Align::Center);
                vbox.set_hexpand(true);
                vbox.set_vexpand(true);
                setup_picture_cell(&window, &picture_grid, &vbox, coords, &repository_rc);
                picture_grid.attach(&vbox, col as i32, row as i32, 1, 1);
            }
        }
        grid_scrolled_window.set_child(Some(&panel));

        let evk = gtk::EventControllerKey::new();
        let graphics = Graphics {
            application_window: window,
            stack: stack,
            grid_scrolled_window: grid_scrolled_window,
            view_scrolled_window: view_scrolled_window,
            picture_grid: picture_grid,
            image_view: image_view,
        };
        let graphics_rc = Rc::new(RefCell::new(graphics));

        evk.connect_key_pressed(clone!(@strong repository_rc, @strong graphics_rc => move |_, key, _, _| {
            let step = 100;
            let graphics = graphics_rc.try_borrow().unwrap();
            let window = &graphics.application_window;
            let picture_grid = &graphics.picture_grid;
            let stack = &graphics.stack;
            let grid_scrolled_window = &graphics.grid_scrolled_window;
            let view_scrolled_window = &graphics.view_scrolled_window;
            let mut show_is_on = true;
            if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                if let Some(key_name) = key.name() {
                    if repository.label_edit_mode_on() {
                        if key_name == "Return" {
                            repository.confirm_label_edit()
                        } else if key_name == "BackSpace" {
                            repository.remove_label_char()
                        } else if key_name == "Escape" {
                            repository.cancel_label_edit()
                        } else {
                            if let Some(ch) = key.to_lower().to_unicode() {
                                match ch {
                                    'a'..='z' => repository.add_label_char(ch),
                                    _ => {} ,
                                }
                            }
                        }
                    } else {
                        match key_name.as_str() {
                            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                                let digit:usize = key_name.parse().unwrap();
                                repository.add_register_digit(digit)
                            },
                            "BackSpace" => repository.delete_register_digit(),
                            "Return" => repository.select_point(),
                            "comma" => repository.point_select(),
                            "Escape" => repository.cancel_point(),
                            "g" => repository.move_to_register(),
                            "j" => repository.move_forward_ten_pages(),
                            "l" => if repository.order_choice_on() { repository.sort_by(Order::Label) } else { repository.move_backward_ten_pages() },
                            "f" => repository.toggle_real_size(),
                            "z" => repository.move_to_index(0),
                            "e" => repository.move_next_page(),
                            "x" => repository.toggle_palette_extract(),
                            "n" => if repository.order_choice_on() { repository.sort_by(Order::Name); } else { repository.move_next_page() },
                            "i" => repository.move_prev_page(),
                            "p" => if repository.order_choice_on() { repository.sort_by(Order::Palette); } else { repository.move_prev_page() },
                            "q" => { repository.quit(); show_is_on = false; window.close() },
                            "Q" => { repository.copy_move_and_quit(&copy_selection_target, &move_selection_target); show_is_on = false; window.close() },
                            "X" => { repository.delete_entries(); show_is_on = false; window.close() },
                            "B" => repository.point_rank(Rank::NoStar),
                            "Eacute" => repository.point_rank(Rank::OneStar),
                            "P" => repository.point_rank(Rank::TwoStars),
                            "O" => repository.point_rank(Rank::ThreeStars),
                            "c" => if repository.order_choice_on() { repository.sort_by(Order::Colors); },
                            "d" => if repository.order_choice_on() { repository.sort_by(Order::Date); },
                            "D" => repository.toggle_delete(),
                            "R" => repository.set_rank(Rank::NoStar),
                            "r" => if repository.order_choice_on() { repository.sort_by(Order::Random); } else { repository.move_to_random_index() },
                            "a" => repository.select_page(true),
                            "u" => repository.select_page(false),
                            "U" => repository.select_all(false),
                            "s" => if repository.order_choice_on() { repository.sort_by(Order::Size); } else { repository.save_select_entries() },
                            "equal" => repository.set_order_choice_on(),
                            "slash" => repository.begin_label_edit(),
                            "minus" => repository.point_remove_label(),
                            "asterisk" => repository.apply_last_label(),
                            "plus" => repository.point_label(),
                            "v" => if repository.order_choice_on() { repository.sort_by(Order::Value); },
                            "h" => repository.help(),
                            "period"|"k" => {
                                if stack.visible_child().unwrap() == *grid_scrolled_window {
                                    stack.set_visible_child(view_scrolled_window);
                                    setup_picture_view(&repository_rc, &window);
                                } else {
                                    stack.set_visible_child(grid_scrolled_window)
                               }
                            },
                            "colon" => {
                                println!("{}", repository.title_display());
                                println!("{}", repository.current_entry().expect("can't access current entry").original_file_path())
                            },
                            "space" => repository.move_next_page(),
                            "Right" => {
                                show_is_on = !repository.real_size();
                                if repository.real_size() {
                                    let h_adj = picture_hadjustment(&window);
                                    h_adj.set_value(h_adj.value() + step as f64)
                                } else {
                                    if repository.cells_per_row() == 1 {
                                        repository.move_next_page();
                                    } else {
                                        navigate(&mut repository, &picture_grid, &window, Direction::Right);
                                        if stack.visible_child().unwrap() == *view_scrolled_window {
                                            setup_picture_view(&repository_rc, &window)
                                        }
                                    }
                                }
                            },
                            "Left" => {
                                show_is_on = !repository.real_size();
                                if repository.real_size() {
                                    let h_adj = picture_hadjustment(&window);
                                    h_adj.set_value(h_adj.value() - step as f64)
                                } else {
                                    if repository.cells_per_row() == 1 {
                                        repository.move_prev_page();
                                    } else {
                                        navigate(&mut repository, &picture_grid, &window, Direction::Left);
                                        if stack.visible_child().unwrap() == *view_scrolled_window {
                                            setup_picture_view(&repository_rc, &window)
                                        }
                                    }
                                }
                            },
                            "Down" => {
                                show_is_on = !repository.real_size();
                                if repository.real_size() {
                                    let v_adj = picture_vadjustment(&window);
                                    v_adj.set_value(v_adj.value() + step as f64)
                                } else {
                                    if repository.cells_per_row() == 1 {
                                        repository.move_next_page()
                                    } else {
                                        navigate(&mut repository, &picture_grid, &window, Direction::Down);
                                        if stack.visible_child().unwrap() == *view_scrolled_window {
                                            setup_picture_view(&repository_rc, &window)
                                        }
                                    }
                                }
                            },
                            "Up" => {
                                show_is_on = !repository.real_size();
                                if repository.real_size() {
                                    let v_adj = picture_vadjustment(&window);
                                    v_adj.set_value(v_adj.value() - step as f64)
                                } else {
                                    if repository.cells_per_row() == 1 {
                                        repository.move_next_page();
                                    } else {
                                        navigate(&mut repository, &picture_grid, &window, Direction::Up);
                                        if stack.visible_child().unwrap() == *view_scrolled_window {
                                            setup_picture_view(&repository_rc, &window)
                                        }
                                    }
                                }
                            },
                            other => println!("{}", other),
                        }
                    };
                }
            }
            if show_is_on {
                if stack.visible_child().unwrap() == *grid_scrolled_window {
                    setup_picture_grid(&repository_rc, &window)
                } else {
                    setup_picture_view(&repository_rc, &window)
                }
            }
            gtk::Inhibit(false)
        }));
        if let Ok(graphics) = graphics_rc.try_borrow() {
            let window = &graphics.application_window;
            let picture_grid = &graphics.picture_grid;
            window.add_controller(evk);

            // show the first file
            if args.maximized { window.fullscreen() };
            // if a timer has been passed, set a timeout routine
            if let Some(t) = args.timer {
                timeout_add_local(Duration::new(t,0), clone!(@strong repository_rc, @strong picture_grid, @strong window => move | | {
                    {
                        let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                        repository.move_next_page();
                    }
                    setup_picture_grid(&repository_rc, &window);
                    Continue(true)
                }));
            };

            setup_picture_grid(&repository_rc, &window);
            window.present();
        };
    }));
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}

fn picture_grid(window: &gtk::ApplicationWindow) -> gtk::Grid {
    let stack = window.first_child().expect("can't access to stack")
        .downcast::<gtk::Stack>().expect("can't downcast to Stack");
    let grid_scrolled_window = stack.first_child().expect("can't access to grid_scrolled_window")
        .downcast::<gtk::ScrolledWindow>().expect("can't downcast to ScrolledWindow");
    let view_port = grid_scrolled_window.first_child().expect("can't access to view_port")
        .downcast::<gtk::Viewport>().expect("can't downcast to Viewport");
    let panel = view_port.first_child().expect("can't access to panel")
        .downcast::<gtk::Grid>().expect("can't downcast to Grid");
    let child = panel.first_child().expect("can't access panel first child");
    if child.widget_name() == "picture_grid" {
        child.downcast::<gtk::Grid>().expect("can't downcast to Grid")
    } else {
        let next = child.next_sibling().expect("can't access panel second child");
        if next.widget_name() == "picture_grid" {
            next.downcast::<gtk::Grid>().expect("can't downcast to Grid")
        } else {
            panic!("can't access to picture grid")
        }
    }
}

fn picture_view(window: &gtk::ApplicationWindow) -> gtk::Grid {
    let stack = window.first_child().expect("can't access to stack")
        .downcast::<gtk::Stack>().expect("can't downcast to Stack");
    let grid_scrolled_window = stack.first_child().expect("can't access to grid_scrolled_window")
        .downcast::<gtk::ScrolledWindow>().expect("can't downcast to ScrolledWindow");
    let view_scrolled_window = grid_scrolled_window.next_sibling().expect("can't access to view_scrollde_window")
        .downcast::<gtk::ScrolledWindow>().expect("can't downcast to ScrolledWindow");
    let view_port = view_scrolled_window.first_child().expect("can't access to view_port")
        .downcast::<gtk::Viewport>().expect("can't downcast to Viewport");
    let view = view_port.first_child().expect("can't access to view")
        .downcast::<gtk::Grid>().expect("can't downcast to Grid");
    view
}

fn setup_picture_grid(repository_rc: &Rc<RefCell<Repository>>, window: &gtk::ApplicationWindow) {
    let picture_grid = picture_grid(window);
    if let Ok(repository) = repository_rc.try_borrow() {
        let cells_per_row = repository.cells_per_row();
        for col in 0..cells_per_row {
            for row in 0..cells_per_row {
                let vbox = picture_grid.child_at(col,row).unwrap().downcast::<gtk::Box>().unwrap();
                setup_picture_cell(window, &picture_grid, &vbox, (col, row), &repository_rc);
            }
        }
        window.set_title(Some(&repository.title_display()));
    }
    else {
        println!("can't borrow repository_rc");
    }
}

fn setup_picture_view(repository_rc: &Rc<RefCell<Repository>>, window: &gtk::ApplicationWindow) {
    let picture_view = picture_view(window);
    if let Ok(repository) = repository_rc.try_borrow() {
        let entry = repository.current_entry().unwrap();
        let picture = picture_view.first_child().unwrap().downcast::<gtk::Picture>().unwrap();
        match set_original_picture_file(&picture, &entry) {
            Ok(_) => {
                window.set_title(Some(&repository.title_display()))
            },
            Err(err) => {
                picture.set_visible(false);
                println!("{}",err.to_string())
            },
        }
    }
}

fn picture_hadjustment(window: &gtk::ApplicationWindow) -> gtk::Adjustment {
    window
        .child()
        .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
        .and_then(|child| child.downcast::<ScrolledWindow>().ok())
        .and_then(|sw| Some(sw.hadjustment()))
        .expect("Failed to get hadjustment").clone()
}
fn picture_vadjustment(window: &gtk::ApplicationWindow) -> gtk::Adjustment {
    window
        .child()
        .and_then(|child| child.downcast::<gtk::Stack>().unwrap().visible_child())
        .and_then(|child| child.downcast::<ScrolledWindow>().ok())
        .and_then(|sw| Some(sw.vadjustment()))
        .expect("Failed to get vadjustment").clone()
}

fn show_view(grid: &Grid, repository: &Repository, window: &gtk::ApplicationWindow) {
    let entry = repository.current_entry().unwrap();
    let picture = grid.first_child().unwrap().downcast::<gtk::Picture>().unwrap();
    match set_original_picture_file(&picture, &entry) {
        Ok(_) => {
            window.set_title(Some(&repository.title_display()))
        },
        Err(err) => {
            picture.set_visible(false);
            println!("{}",err.to_string())
        },
    }
}

fn label_at_coords(grid: &gtk::Grid, coords: Coords) -> Option<gtk::Label> {
    let (col,row) = coords;
    let vbox = grid.child_at(col as i32, row as i32).expect("can't find a child").downcast::<gtk::Box>().expect("can't downcast child to a Box");
    let child = vbox.first_child().expect("can't access vbox first child").downcast::<gtk::Picture>().expect("can't downcast to Picture");
    let next = child.next_sibling().expect("can't access vbox next child");
    if next.widget_name() == "picture_label" {
        Some(next.downcast::<gtk::Label>().unwrap())
    } else {
        let next_next = next.next_sibling().expect("can't access vbox next next child");
        if next_next.widget_name() == "picture_label" {
            Some(next_next.downcast::<gtk::Label>().unwrap())
        } else {
            panic!("can't find grid picture label");
        }
    }
}

fn set_label_text_at_coords(grid: &gtk::Grid, coords: Coords, text: String) {
    if let Some(label) = label_at_coords(grid, coords) {
        label.set_text(&text)
    }
}

fn navigate(repository: &mut Repository, grid: &gtk::Grid, window: &gtk::ApplicationWindow, direction: Direction) {
    if repository.can_move_rel(direction.clone()) {
        if let Some(current_label) = label_at_coords(&grid, repository.position()) {
            let current_display = match repository.current_entry() {
                Some(entry) => entry.label_display(false),
                None => String::new(),
            };
            current_label.set_text(&current_display);
        }
        repository.move_rel(direction);
        if let Some(new_label) = label_at_coords(&grid, repository.position()) {
            let new_display = match repository.current_entry() {
                Some(entry) => entry.label_display(true),
                None => String::new(),
            };
            new_label.set_text(&new_display);
        }
        window.set_title(Some(&repository.title_display()));
    }
}

fn setup_picture_cell(window: &gtk::ApplicationWindow, grid: &gtk::Grid, vbox: &gtk::Box, coords: Coords, repository_rc: &Rc<RefCell<Repository>>) {
    while let Some(child) = vbox.first_child() {
        vbox.remove(&child)
    };
    if let Ok(repository) = repository_rc.try_borrow() {
        if let Some(index) = repository.index_from_position(coords) {
            if let Some(entry) = repository.entry_at_index(index) {
                let picture = gtk::Picture::new();
                let opacity = if entry.delete { 0.25 }
                else if entry.image_data.selected { 0.50 } else { 1.0 };
                picture.set_valign(Align::Center);
                picture.set_halign(Align::Center);
                picture.set_opacity(opacity);
                picture.set_can_shrink(!repository.real_size());
                let result = if repository.cells_per_row() < 10 {
                    set_original_picture_file(&picture, &entry)
                } else {
                    set_thumbnail_picture_file(&picture, &entry)
                };
                match result {
                    Ok(_) => picture.set_visible(true),
                    Err(err) => {
                        picture.set_visible(false);
                        println!("{}", err.to_string())
                    },
                };
                let is_current_entry = index == repository.current_index() && repository.cells_per_row() > 1;
                let label = gtk::Label::new(Some(&entry.label_display(is_current_entry)));
                label.set_valign(Align::Center);
                label.set_halign(Align::Center);
                label.set_widget_name("picture_label");
                vbox.append(&picture);
                if repository.palette_extract_on() { 
                    let drawing_area = gtk::DrawingArea::new();
                    drawing_area.set_valign(Align::Center);
                    drawing_area.set_halign(Align::Center);
                    let colors = entry.image_data.palette;
                    drawing_area.set_content_width(90);
                    drawing_area.set_content_height(10);
                    drawing_area.set_hexpand(true);
                    drawing_area.set_vexpand(true);
                    drawing_area.set_draw_func(move |_, ctx, _, _| {
                        draw_palette(ctx, 90, 10, &colors)
                    });
                    vbox.append(&drawing_area);
                }
                let motion_controller = gtk::EventControllerMotion::new();
                motion_controller.connect_leave(clone!(@strong coords, @strong label, @strong entry, => move |_| {
                    label.set_text(&entry.label_display(false));
                }));
                motion_controller.connect_enter(clone!(@strong coords, @strong label, @strong entry, @strong repository_rc, @strong window => move |_,_,_| {
                    if let Ok(repository) = repository_rc.try_borrow_mut() {
                    }
                }));
                let gesture_left_click = gtk::GestureClick::new();
                gesture_left_click.set_button(1);
                gesture_left_click.connect_pressed(clone!(@strong coords, @strong label, @strong entry, @strong repository_rc, @strong window, @strong grid => move |_,_,_,_| {
                    if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                        if repository.cells_per_row() > 1 {
                            if repository.can_move_abs(coords) {
                                let current_coords = repository.position();
                                if let Some(index) = repository.index_from_position(current_coords) {
                                    if let Some(current_entry) = repository.entry_at_index(index) {
                                        set_label_text_at_coords(&grid, current_coords, current_entry.label_display(false))
                                    }
                                };
                                repository.move_abs(coords);
                                if let Some(entry) = repository.current_entry() {
                                    label.set_text(&entry.label_display(true));
                                }

                                window.set_title(Some(&(repository.title_display())));
                            }
                        }
                    }
                }));
                picture.add_controller(gesture_left_click);
                let gesture_right_click = gtk::GestureClick::new();
                gesture_right_click.set_button(3);
                gesture_right_click.connect_pressed(clone!(@strong coords, @strong label, @strong repository_rc, @strong window, @strong grid => move |_,_,_,_| {
                    // label.set_text(&entry.label_display(true));
                    if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                        if repository.cells_per_row() > 1 {
                            if repository.can_move_abs(coords) {
                                let current_coords = repository.position();
                                if let Some(index) = repository.index_from_position(current_coords) {
                                    if let Some(current_entry) = repository.entry_at_index(index) {
                                        set_label_text_at_coords(&grid, current_coords, current_entry.label_display(false))
                                    }
                                };
                                repository.move_abs(coords);
                                repository.select_point();
                                if let Some(entry) = repository.current_entry() {
                                    label.set_text(&entry.label_display(true));
                                }

                                window.set_title(Some(&(repository.title_display())));
                            }
                        }
                    }
                }));
                picture.add_controller(gesture_right_click);

                vbox.append(&label);
            }
        }
    } else {
        println!("can't borrow repository_rc");
    }

}
