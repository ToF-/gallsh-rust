use clap::Parser;
use crate::args::{Args, grid_size, height, selection_target, width};
use crate::direction::Direction;
use crate::navigator::Coords;
use crate::paths::determine_path;
use crate::picture_io::{draw_palette, ensure_thumbnail, is_valid_path, read_entries, set_original_picture_file, set_thumbnail_picture_file};
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
use std::process;
use std::rc::Rc;
use std::time::{Duration};
use crate::graphics::{command, setup_image_view, setup_picture_cell, setup_picture_grid};
use crate::graphics::Graphics;



mod args;
mod direction;
mod entry;
mod graphics;
mod image;
mod image_data;
mod navigator;
mod order;
mod paths;
mod picture_io;
mod rank;
mod repository;



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
        let width = width(args.width);
        let height = height(args.height);
        let copy_selection_target = match selection_target(&args.copy_selection) {
            Ok(target) => target,
            Err(_) => process::exit(1),
        };
        let move_selection_target = match selection_target(&args.move_selection) {
            Ok(target) => target,
            Err(_) => process::exit(1),
        };

        let grid_size = grid_size(args.thumbnails, args.grid);

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

        let application_window = gtk::ApplicationWindow::builder()
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

        application_window.set_child(Some(&stack));

        let image_view = Picture::new();
        let view_gesture = gtk::GestureClick::new();
        view_gesture.set_button(0);
        view_gesture.connect_pressed(clone!(@strong repository_rc, @strong stack, @strong grid_scrolled_window, @strong application_window => move |_,_, _, _| {
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
        left_gesture.connect_pressed(clone!(@strong repository_rc, @strong picture_grid, @strong picture_grid, @strong application_window => move |_,_,_,_| {
            {
                let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                repository.move_prev_page();
            }
            setup_picture_grid(&repository_rc, &picture_grid, &application_window);
        }));
        left_button.add_controller(left_gesture);
        let right_gesture = gtk::GestureClick::new();
        right_gesture.set_button(1);
        right_gesture.connect_pressed(clone!(@strong repository_rc, @strong picture_grid, @strong application_window => move |_,_,_,_| {
            {
                let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                repository.move_next_page();
            }
            setup_picture_grid(&repository_rc, &picture_grid, &application_window);
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
                setup_picture_cell(&application_window, &picture_grid, &vbox, coords, &repository_rc);
                picture_grid.attach(&vbox, col as i32, row as i32, 1, 1);
            }
        }
        grid_scrolled_window.set_child(Some(&panel));

        let graphics = Graphics {
            application_window: application_window,
            stack: stack,
            grid_scrolled_window: grid_scrolled_window,
            view_scrolled_window: view_scrolled_window,
            picture_grid: picture_grid,
            image_view: image_view,
        };
        let graphics_rc = Rc::new(RefCell::new(graphics));

        let evk = gtk::EventControllerKey::new();

        evk.connect_key_pressed(clone!(@strong repository_rc, @strong graphics_rc => move |_, key, _, _| {
            let graphics = graphics_rc.try_borrow().unwrap();
            let mut refresh = true;
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
                            "q" => { repository.quit(); refresh = false; graphics.application_window.close() },
                            "Q" => { repository.copy_move_and_quit(&copy_selection_target, &move_selection_target); refresh = false; graphics.application_window.close() },
                            "X" => { repository.delete_entries(); refresh = false; graphics.application_window.close() },
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
                                if graphics.view_mode() {
                                    graphics.stack.set_visible_child(&graphics.grid_scrolled_window)
                                } else {
                                    graphics.stack.set_visible_child(&graphics.view_scrolled_window);
                                    setup_image_view(&repository_rc, &graphics.image_view, &graphics.application_window)
                                }
                            },
                            "colon" => {
                                println!("{}", repository.title_display());
                                println!("{}", repository.current_entry().expect("can't access current entry").original_file_path())
                            },
                            "space" => repository.move_next_page(),
                            "Right" => {
                                refresh = !repository.real_size();
                                command(Direction::Right, &graphics, &mut repository, &repository_rc)
                            },
                            "Left" => {
                                refresh = !repository.real_size();
                                command(Direction::Left, &graphics, &mut repository, &repository_rc)
                            },
                            "Down" => {
                                refresh = !repository.real_size();
                                command(Direction::Down, &graphics, &mut repository, &repository_rc)
                            },
                            "Up" => {
                                refresh = !repository.real_size();
                                command(Direction::Up, &graphics, &mut repository, &repository_rc)
                            },
                            other => println!("{}", other),
                        }
                    };
                }
            }
            if refresh {
                if graphics.stack.visible_child().unwrap() == graphics.grid_scrolled_window {
                    setup_picture_grid(&repository_rc, &graphics.picture_grid, &graphics.application_window)
                } else {
                    setup_image_view(&repository_rc, &graphics.image_view, &graphics.application_window)
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
                    setup_picture_grid(&repository_rc, &picture_grid, &window);
                    Continue(true)
                }));
            };

            setup_picture_grid(&repository_rc, &picture_grid, &window);
            window.present();
        };
    }));
    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}


