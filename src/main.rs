use crate::picture_io::draw_palette;
use clap::Parser;
use gtk::DrawingArea;
use crate::args::Args;
use crate::direction::Direction;
use crate::navigator::Coords;
use crate::paths::determine_path;
use crate::picture_io::ensure_thumbnail;
use crate::picture_io::{read_entries, set_original_picture_file, set_thumbnail_picture_file};
use crate::repository::Repository;
use entry::{Entry, EntryList, make_entry};
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::EventControllerMotion;
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
            Some(target) => Some(target.to_string()),
            None => None,
        };

        let move_selection_target: Option<String> = match &args.move_selection {
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

        let order = Order::from_options(args.name, args.date, args.size, args.colors, args.value, args.palette);
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

        let grid = Grid::new();
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(true);
        grid.set_hexpand(true);
        grid.set_vexpand(true);
        if grid_size > 1 {
            panel.attach(&left_button, 0, 0, 1, 1);
            panel.attach(&grid, 1, 0, 1, 1);
            panel.attach(&right_button, 2, 0, 1, 1);
        } else {
            panel.attach(&grid, 0, 0, 1, 1);
        }
        left_gesture.set_button(1);
        left_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong window => move |_,_,_,_| {
            let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
            repository.move_prev_page();
            show_grid(&grid, &repository, &window);
        }));
        left_button.add_controller(left_gesture);
        let right_gesture = gtk::GestureClick::new();
        right_gesture.set_button(1);
        right_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong window => move |_,_,_,_| {
            let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
            repository.move_next_page();
            show_grid(&grid, &repository, &window);
        }));
        right_button.add_controller(right_gesture);
        for col in 0 .. grid_size as i32 {
            for row in 0 .. grid_size as i32 {
                let coords: Coords = (col,row);
                let vbox = gtk::Box::new(Orientation::Vertical, 0);
                let image = Picture::new();
                image.set_hexpand(true);
                image.set_vexpand(true);
                let label = Label::new(None);
                let drawing_area = DrawingArea::new();
                let style_context = label.style_context();
                style_context.add_provider(&buttons_css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
                vbox.set_valign(Align::Center);
                vbox.set_halign(Align::Center);
                vbox.set_hexpand(true);
                vbox.set_vexpand(true);
                vbox.append(&image);
                vbox.append(&label);
                vbox.append(&drawing_area);
                grid.attach(&vbox, col as i32, row as i32, 1, 1);

                let select_gesture = gtk::GestureClick::new();
                select_gesture.set_button(1);
                select_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong window => move |_,_, _, _| {
                    let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                    if repository.can_move_abs(coords) {
                        repository.move_abs(coords);
                        repository.select_point();
                    }
                    show_grid(&grid, &repository, &window);
                }));
                image.add_controller(select_gesture);

                let view_gesture = gtk::GestureClick::new();
                view_gesture.set_button(3);

                view_gesture.connect_pressed(clone!(@strong repository_rc, @strong grid, @strong image, @strong view, @strong stack, @strong view_scrolled_window, @strong grid_scrolled_window, @strong window => move |_, _, _, _| {
                    let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                    if repository.cells_per_row() == 1 { return };
                    if repository.can_move_abs(coords) {
                        repository.move_abs(coords);
                        repository.select_point();
                        stack.set_visible_child(&view_scrolled_window);
                        show_view(&view, &repository, &window);
                    }
                }));
                image.add_controller(view_gesture);

                let motion_controller = EventControllerMotion::new();
                motion_controller.connect_enter(clone!(@strong repository_rc, @strong grid, @strong label, @strong window => move |_,_,_| {
                    if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                        if repository.can_move_abs(coords) {
                            repository.move_abs(coords);
                            if let Some(entry) = repository.current_entry() {
                                label.set_text(&entry.label_display(true))
                            };
                            window.set_title(Some(&(repository.title_display())));
                        } else {
                            println!("{:?} refused", coords)
                        }
                    }
                }));

                motion_controller.connect_leave(clone!(@strong repository_rc, @strong grid, @strong label, @strong window => move |_| {
                    if let Ok(repository) = repository_rc.try_borrow_mut() {
                        if let Some(index) = repository.index_from_position(coords) {
                            if let Some(entry) = repository.entry_at_index(index) {
                                label.set_text(&entry.label_display(false));
                            }
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
                    //if stack.visible_child().unwrap() == view_scrolled_window {
                    //    stack.set_visible_child(&grid_scrolled_window);
                    //    return gtk::Inhibit(false)
                    //};
                    let mut show_is_on = true;
                    match s.as_str() {
                        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                            let digit:usize = s.parse().unwrap();
                            repository.add_register_digit(digit)
                        },
                        "BackSpace" => repository.delete_register_digit(),
                        "Return" => repository.select_point(),
                        "comma" => repository.point_select(),
                        "Escape" => repository.cancel_point(),
                        "g" => repository.move_to_register(),
                        "j" => repository.move_forward_ten_pages(),
                        "l" => repository.move_backward_ten_pages(),
                        "f" => repository.toggle_real_size(),
                        "z" => repository.move_to_index(0),
                        "e" => repository.move_next_page(),
                        "x" => repository.toggle_palette_extract(),
                        "n" => if repository.order_choice_on() { repository.sort_by(Order::Name); } else { repository.move_next_page() },
                        "i" => repository.move_prev_page(),
                        "p" => if repository.order_choice_on() { repository.sort_by(Order::Palette); } else { repository.move_prev_page() },
                        "q" => { repository.quit(); show_is_on = false; window.close() },
                        "Q" => { repository.copy_move_and_quit(&copy_selection_target, &move_selection_target); show_is_on = false; window.close() },
                        "B"|"plus"|"D" => repository.point_rank(Rank::NoStar),
                        "M"|"Eacute"|"minus"|"C" => repository.point_rank(Rank::OneStar),
                        "N"|"P"|"slash" => repository.point_rank(Rank::TwoStars),
                        "asterisk"|"A"|"O" => repository.point_rank(Rank::ThreeStars),
                        "c" => if repository.order_choice_on() { repository.sort_by(Order::Colors); },
                        "d" => if repository.order_choice_on() { repository.sort_by(Order::Date); },
                        "R" => repository.set_rank(Rank::NoStar),
                        "r" => if repository.order_choice_on() { repository.sort_by(Order::Random); } else { repository.move_to_random_index() },
                        "a" => repository.select_page(true),
                        "u" => repository.select_page(false),
                        "U" => repository.select_all(false),
                        "s" => if repository.order_choice_on() { repository.sort_by(Order::Size); } else { repository.save_select_entries() },
                        "equal" => repository.set_order_choice_on(),
                        "v" => if repository.order_choice_on() { repository.sort_by(Order::Value); },
                        "h" => repository.help(),
                        "period"|"k" => {
                            if stack.visible_child().unwrap() == grid_scrolled_window {
                                stack.set_visible_child(&view_scrolled_window);
                                show_view(&view, &repository, &window);
                            } else {
                                stack.set_visible_child(&grid_scrolled_window)
                            }
                        },
                        "colon" => {
                            println!("{}", repository.title_display());
                            println!("{}", repository.current_entry().expect("can't access current entry").original_file_path())
                        },
                        "space" => repository.move_next_page(),
                        "Right" => {
                            show_is_on = repository.cells_per_row() == 1 && !repository.real_size();
                            if repository.real_size() {
                                let h_adj = picture_hadjustment(&window);
                                h_adj.set_value(h_adj.value() + step as f64)
                            } else {
                                if repository.cells_per_row() == 1 {
                                    repository.move_next_page();
                                } else {
                                    navigate(&mut repository, &grid, &window, Direction::Right);
                                    if stack.visible_child().unwrap() == view_scrolled_window {
                                        show_view(&view, &repository, &window)
                                    }
                                }
                            }
                        },
                        "Left" => {
                            show_is_on = repository.cells_per_row() == 1 && !repository.real_size();
                            if repository.real_size() {
                                let h_adj = picture_hadjustment(&window);
                                h_adj.set_value(h_adj.value() - step as f64)
                            } else {
                                if repository.cells_per_row() == 1 {
                                    repository.move_prev_page();
                                } else {
                                    navigate(&mut repository, &grid, &window, Direction::Left);
                                    if stack.visible_child().unwrap() == view_scrolled_window {
                                        show_view(&view, &repository, &window)
                                    }
                                }
                            }
                        },
                        "Down" => {
                            show_is_on = repository.cells_per_row() == 1 && !repository.real_size();
                            if repository.real_size() {
                                let v_adj = picture_vadjustment(&window);
                                v_adj.set_value(v_adj.value() + step as f64)
                            } else {
                                if repository.cells_per_row() == 1 {
                                    repository.move_next_page()
                                } else {
                                    navigate(&mut repository, &grid, &window, Direction::Down);
                                    if stack.visible_child().unwrap() == view_scrolled_window {
                                        show_view(&view, &repository, &window)
                                    }
                                }
                            }
                        },
                        "Up" => {
                            show_is_on = repository.cells_per_row() == 1 && !repository.real_size();
                            if repository.real_size() {
                                let v_adj = picture_vadjustment(&window);
                                v_adj.set_value(v_adj.value() - step as f64)
                            } else {
                                if repository.cells_per_row() == 1 {
                                    repository.move_next_page();
                                } else {
                                    navigate(&mut repository, &grid, &window, Direction::Up);
                                    if stack.visible_child().unwrap() == view_scrolled_window {
                                        show_view(&view, &repository, &window)
                                    }
                                }
                            }
                        },
                        other => println!("{}", other), 
                    };
                    if show_is_on {
                        if stack.visible_child().unwrap() == grid_scrolled_window {
                            show_grid(&grid, &repository, &window)
                        } else {
                            show_view(&view, &repository, &window)
                        }
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
                repository.move_next_page();
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
    let cells_per_row = repository.cells_per_row();
    for col in 0..cells_per_row {
        for row in 0..cells_per_row {
            let vbox = grid.child_at(col,row).unwrap().downcast::<gtk::Box>().unwrap();
            vbox.set_hexpand(true);
            let picture = vbox.first_child().unwrap().downcast::<gtk::Picture>().unwrap();
            let label = picture.next_sibling().unwrap().downcast::<gtk::Label>().unwrap();
            let palette = match repository.palette_extract() {
                true => Some(label.next_sibling().unwrap().downcast::<gtk::DrawingArea>().unwrap()),
                false => None,
            };
            if let Some(index) = repository.index_from_position((col,row)) {
                if let Some(entry) = repository.entry_at_index(index) {
                    let status = format!("{} {} {}",
                        if index == repository.index() && cells_per_row > 1 { "▄" } else { "" },
                        entry.image_data.rank.show(),
                        if entry.image_data.selected { "△" } else { "" });
                    label.set_text(&status);
                    let opacity = if entry.image_data.selected { 0.50 } else { 1.0 };
                    picture.set_opacity(opacity);
                    picture.set_can_shrink(!repository.real_size());
                    if repository.cells_per_row() < 10 {
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
                    if let Some(drawing_area) = palette {
                        let colors = entry.image_data.palette;
                        let allocation = vbox.allocation();
                        let width = allocation.width()/2;
                        let height = allocation.height()/10;
                        drawing_area.set_content_width(width);
                        drawing_area.set_content_height(height);
                        drawing_area.set_draw_func(move |_,ctx,_,_| {
                            draw_palette(ctx, width, height, &colors)
                        });
                    };
                }
            } else {
                picture.set_visible(false);
                label.set_text("");
            }
        }
    }
    window.set_title(Some(&repository.title_display()));
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

fn label_at(grid: &gtk::Grid, coords: Coords) -> gtk::Label {
    let (col,row) = coords;
    grid.child_at(col as i32, row as i32).unwrap()
        .downcast::<gtk::Box>().unwrap()
        .first_child().unwrap().downcast::<gtk::Picture>().unwrap()
        .next_sibling().unwrap().downcast::<gtk::Label>().unwrap()
}

fn navigate(repository: &mut Repository, grid: &gtk::Grid, window: &gtk::ApplicationWindow, direction: Direction) {
    if repository.can_move_rel(direction.clone()) {
        let old_coords = repository.position();
        let old_label = label_at(&grid, old_coords);
        let old_display = match repository.current_entry() {
            Some(entry) => entry.label_display(false),
            None => String::new(),
        };
        old_label.set_text(&old_display);
        repository.move_rel(direction);
        let new_coords = repository.position();
        let new_label = label_at(&grid, new_coords);
        let new_display = match repository.current_entry() {
            Some(entry) => entry.label_display(true),
            None => String::new(),
        };
        new_label.set_text(&new_display);
        window.set_title(Some(&repository.title_display()));
    }
}
