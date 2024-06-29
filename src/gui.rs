use crate::repository::init_repository;
use crate::picture_io::move_entries_with_label;
use crate::timeout_add_local;
use crate::ensure_thumbnails;
use crate::read_entries;
use crate::Args;
use std::time::Duration;
use crate::Rank;
use crate::Order;
use gtk::gdk::Key;
use gtk::Orientation;
use std::cell::RefMut;
use gtk::Picture;
use gtk::Label;
use gtk::Grid;
use gtk::CssProvider;
use crate::direction::Direction;
use crate::navigator::Coords;
use crate::picture_io::{draw_palette, set_original_picture_file, set_thumbnail_picture_file};
use crate::repository::Repository;
use crate::entry::Entry;
use gtk::glib::clone;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use gtk::traits::WidgetExt;
use gtk::{self, Align, ScrolledWindow};
use std::cell::{RefCell};
use std::rc::Rc;

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

pub struct Gui {
    pub application_window:   gtk::ApplicationWindow,
    pub stack:                gtk::Stack,
    pub grid_scrolled_window: gtk::ScrolledWindow,
    pub view_scrolled_window: gtk::ScrolledWindow,
    pub picture_grid:       gtk::Grid,
    pub image_view:         gtk::Picture,
}

impl Gui {

    pub fn view_mode(&self) -> bool {
        self.stack.visible_child().unwrap() == self.view_scrolled_window
    }
}


pub fn setup_picture_grid(repository_rc: &Rc<RefCell<Repository>>, picture_grid: &gtk::Grid, window: &gtk::ApplicationWindow) {
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

pub fn setup_image_view(repository_rc: &Rc<RefCell<Repository>>, picture_view: &gtk::Picture, window: &gtk::ApplicationWindow) {
    if let Ok(repository) = repository_rc.try_borrow() {
        let entry = repository.current_entry().unwrap();
        match set_original_picture_file(&picture_view, &entry) {
            Ok(_) => {
                window.set_title(Some(&repository.title_display()))
            },
            Err(err) => {
                picture_view.set_visible(false);
                println!("{}",err.to_string())
            },
        }
    }
}

pub fn picture_hadjustment(window: &gtk::ApplicationWindow) -> gtk::Adjustment {
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

pub fn label_at_coords(grid: &gtk::Grid, coords: Coords) -> Option<gtk::Label> {
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

pub fn set_label_text_at_coords(grid: &gtk::Grid, coords: Coords, text: String) {
    if let Some(label) = label_at_coords(grid, coords) {
        label.set_text(&text)
    }
}

pub fn navigate(repository: &mut Repository, grid: &gtk::Grid, window: &gtk::ApplicationWindow, direction: Direction) {
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

pub fn picture_for_entry(entry: &Entry, repository: &Repository) -> gtk::Picture {
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
    picture
}

pub fn label_for_entry(entry: &Entry, index: usize, repository: &Repository) -> gtk::Label {
    let is_current_entry = index == repository.current_index() && repository.cells_per_row() > 1;
    let label = gtk::Label::new(Some(&entry.label_display(is_current_entry)));
    label.set_valign(Align::Center);
    label.set_halign(Align::Center);
    label.set_widget_name("picture_label");
    label
}

pub fn drawing_area_for_entry(entry: &Entry) -> gtk::DrawingArea {
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
    drawing_area
}

pub fn set_label_text_at_current_position(grid: &gtk::Grid, repository: &Repository, has_focus: bool) {
    let current_coords = repository.position();
    if let Some(current_entry) = repository.current_entry() {
        set_label_text_at_coords(grid, current_coords, current_entry.label_display(has_focus))
    };
}

pub fn focus_on_cell_at_coords(coords: Coords, grid: &gtk::Grid, window: &gtk::ApplicationWindow, repository: &mut Repository, with_select: bool) {
    if repository.cells_per_row() > 1 {
        if repository.can_move_abs(coords) {
            set_label_text_at_current_position(&grid, &repository, false);
            repository.move_abs(coords);
            if with_select {
                repository.select_point();
            }
            set_label_text_at_current_position(&grid, &repository, true);
            window.set_title(Some(&(repository.title_display())));
        }
    }
}


pub fn create_gui(application: &gtk::Application, width: i32, height: i32, grid_size: usize, repository_rc: &Rc<RefCell<Repository>>) -> Gui {
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

        let gui = Gui {
            application_window: application_window,
            stack: stack,
            grid_scrolled_window: grid_scrolled_window,
            view_scrolled_window: view_scrolled_window,
            picture_grid: picture_grid,
            image_view: image_view,
        };
        gui
}
pub fn setup_picture_cell(window: &gtk::ApplicationWindow, grid: &gtk::Grid, vbox: &gtk::Box, coords: Coords, repository_rc: &Rc<RefCell<Repository>>) {
    if let Ok(repository) = repository_rc.try_borrow() {
        if let Some(index) = repository.index_from_position(coords) {
            if let Some(entry) = repository.entry_at_index(index) {
                if repository.page_changed() {
                    while let Some(child) = vbox.first_child() {
                        vbox.remove(&child)
                    };
                    let picture = picture_for_entry(entry, &repository);
                    let label = label_for_entry(entry, index, &repository);
                    vbox.append(&picture);
                    if repository.palette_extract_on() { 
                        let drawing_area = drawing_area_for_entry(entry);
                        vbox.append(&drawing_area);
                    }
                    let gesture_left_click = gtk::GestureClick::new();
                    gesture_left_click.set_button(1);
                    gesture_left_click.connect_pressed(clone!(@strong coords, @strong label, @strong entry, @strong repository_rc, @strong window, @strong grid => move |_,_,_,_| {
                        if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                            focus_on_cell_at_coords(coords, &grid, &window, &mut repository, false);
                        }
                    }));
                    picture.add_controller(gesture_left_click);

                    let gesture_right_click = gtk::GestureClick::new();
                    gesture_right_click.set_button(3);
                    gesture_right_click.connect_pressed(clone!(@strong coords, @strong label, @strong repository_rc, @strong window, @strong grid => move |_,_,_,_| {
                        if let Ok(mut repository) = repository_rc.try_borrow_mut() {
                            focus_on_cell_at_coords(coords, &grid, &window, &mut repository, true);
                        }
                    }));
                    picture.add_controller(gesture_right_click);
                    vbox.append(&label);
                }
            }
        } else {
            while let Some(child) = vbox.first_child() {
                vbox.remove(&child)
            }
        }
    } else {
        println!("can't borrow repository_rc");
    }

}

pub fn process_key(repository_rc: &Rc<RefCell<Repository>>, gui_rc: &Rc<RefCell<Gui>>, key: Key) -> gtk::Inhibit {
    let gui = gui_rc.try_borrow().unwrap();
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
                            '0'..='9' => repository.add_label_char(ch),
                            '-'|'_'   => repository.add_label_char(ch),
                            _ => {} ,
                        }
                    }
                }
            } else {
                match key_name.as_str() {
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                        let digit:usize = key_name.parse().unwrap();
                        if repository.register_on() {
                            repository.add_register_digit(digit)
                        } else {
                            match key_name.as_str() {
                                "0" => repository.point_rank(Rank::NoStar),
                                "1" => repository.point_rank(Rank::OneStar),
                                "2" => repository.point_rank(Rank::TwoStars),
                                "3" => repository.point_rank(Rank::ThreeStars),
                                other => println!("{}", other),
                            }
                        }
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
                    "q" => { repository.quit(); refresh = false; gui.application_window.close() },
                    "Q" => { repository.copy_move_and_quit(); refresh = false; gui.application_window.close() },
                    "X" => { repository.delete_entries(); refresh = false; gui.application_window.close() },
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
                        if gui.view_mode() {
                            gui.stack.set_visible_child(&gui.grid_scrolled_window)
                        } else {
                            gui.stack.set_visible_child(&gui.view_scrolled_window);
                            setup_image_view(&repository_rc, &gui.image_view, &gui.application_window)
                        }
                    },
                    "colon" => {
                        println!("{}", repository.title_display());
                        println!("{}", repository.current_entry().expect("can't access current entry").original_file_path())
                    },
                    "space" => repository.move_next_page(),
                    "Right" => {
                        refresh = !repository.real_size();
                        arrow_command(Direction::Right, &gui, &mut repository, &repository_rc)
                    },
                    "Left" => {
                        refresh = !repository.real_size();
                        arrow_command(Direction::Left, &gui, &mut repository, &repository_rc)
                    },
                    "Down" => {
                        refresh = !repository.real_size();
                        arrow_command(Direction::Down, &gui, &mut repository, &repository_rc)
                    },
                    "Up" => {
                        refresh = !repository.real_size();
                        arrow_command(Direction::Up, &gui, &mut repository, &repository_rc)
                    },
                    other => println!("{}", other),
                }
            };
        }
    }
    if refresh {
        if gui.stack.visible_child().unwrap() == gui.grid_scrolled_window {
            setup_picture_grid(&repository_rc, &gui.picture_grid, &gui.application_window)
        } else {
            setup_image_view(&repository_rc, &gui.image_view, &gui.application_window)
        }
    }
    gtk::Inhibit(false)
}
pub fn arrow_command(direction: Direction, gui: &Gui, repository: &mut Repository, repository_rc: &Rc<RefCell<Repository>>) {
    let step: f64 = 100.0;
    let (picture_adjustment, step) = match direction {
        Direction::Right => (picture_hadjustment(&gui.application_window), step),
        Direction::Left  => (picture_hadjustment(&gui.application_window), -step),
        Direction::Down  => (picture_vadjustment(&gui.application_window), step),
        Direction::Up    => (picture_vadjustment(&gui.application_window), -step),
    };
    if repository.real_size() {
        picture_adjustment.set_value(picture_adjustment.value() + step)
    } else {
        if repository.cells_per_row() == 1 {
            repository.move_in_direction(direction)
        } else {
            navigate(repository, &gui.picture_grid, &gui.application_window, direction);
            if gui.stack.visible_child().unwrap() == gui.view_scrolled_window {
                setup_image_view(&repository_rc, &gui.image_view, &gui.application_window)
            }
        }
    }
}


pub fn build_gui(args: &Args, application: &gtk::Application) {
    let width = args.width();
    let height = args.height();
    let grid_size = args.grid_size();
    if let Some(repository) = init_repository(args) {
    let repository_rc = Rc::new(RefCell::new(repository));
    let gui = create_gui(application, width, height, grid_size, &repository_rc);
    let gui_rc = Rc::new(RefCell::new(gui));

    let evk = gtk::EventControllerKey::new();
    evk.connect_key_pressed(clone!(@strong repository_rc, @strong gui_rc => move |_, key, _, _| {
        process_key(&repository_rc, &gui_rc, key) 
    }));
    let gui = gui_rc.try_borrow().unwrap();
    let application_window = &gui.application_window;
    let picture_grid = &gui.picture_grid;
    gui.application_window.add_controller(evk);
    if args.maximized { gui.application_window.fullscreen() };
    // if a timer has been passed, set a timeout routine
    if let Some(t) = args.timer {
        timeout_add_local(Duration::new(t,0), clone!(@strong repository_rc, @strong picture_grid, @strong application_window => move | | {
            {
                let mut repository: RefMut<'_,Repository> = repository_rc.borrow_mut();
                repository.move_next_page();
            }
            setup_picture_grid(&repository_rc, &picture_grid, &application_window);
            Continue(true)
        }));
    };
    setup_picture_grid(&repository_rc, &picture_grid, &application_window);
    application_window.present();
    }
}
