use clap::Parser;
use crate::args::{Args, grid_size, height, selection_target, width};
use crate::direction::Direction;
use crate::navigator::Coords;
use crate::paths::determine_path;
use crate::picture_io::{draw_palette, ensure_thumbnail, is_valid_path, read_entries, set_original_picture_file, set_thumbnail_picture_file};
use crate::repository::Repository;
use crate::entry::{Entry, EntryList, make_entry};
use glib::clone;
use glib::prelude::*;
use glib::timeout_add_local;
use gtk::prelude::*;
use gtk::traits::WidgetExt;
use gtk::{self, Align, Application, CssProvider, Orientation, Label, ScrolledWindow, gdk, glib, Grid, Picture};
use crate::order::{Order};
use crate::paths::THUMB_SUFFIX;
use crate::rank::{Rank};
use std::cell::{RefCell, RefMut};
use std::process;
use std::rc::Rc;
use std::time::{Duration};

pub struct Graphics {
    pub application_window:   gtk::ApplicationWindow,
    pub stack:                gtk::Stack,
    pub grid_scrolled_window: gtk::ScrolledWindow,
    pub view_scrolled_window: gtk::ScrolledWindow,
    pub picture_grid:       gtk::Grid,
    pub image_view:         gtk::Picture,
}

impl Graphics {
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

pub fn command(direction: Direction, graphics: &Graphics, repository: &mut Repository, repository_rc: &Rc<RefCell<Repository>>) {
    let step: f64 = 100.0;
    let (picture_adjustment, step) = match direction {
        Direction::Right => (picture_hadjustment(&graphics.application_window), step),
        Direction::Left  => (picture_hadjustment(&graphics.application_window), -step),
        Direction::Down  => (picture_vadjustment(&graphics.application_window), step),
        Direction::Up    => (picture_vadjustment(&graphics.application_window), -step),
    };
    if repository.real_size() {
        picture_adjustment.set_value(picture_adjustment.value() + step)
    } else {
        if repository.cells_per_row() == 1 {
            repository.move_in_direction(direction)
        } else {
            navigate(repository, &graphics.picture_grid, &graphics.application_window, direction);
            if graphics.stack.visible_child().unwrap() == graphics.view_scrolled_window {
                setup_image_view(&repository_rc, &graphics.image_view, &graphics.application_window)
            }
        }
    }
}
