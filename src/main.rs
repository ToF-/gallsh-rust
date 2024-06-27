use crate::gui::build_gui;
use crate::args::Args;
use crate::direction::Direction;
use crate::paths::determine_path;
use crate::picture_io::{ensure_thumbnails, is_valid_path, read_entries};
use entry::Entry;
use glib::clone;
use glib::timeout_add_local;
use gtk::prelude::*;
use gtk::{self, Application, gdk, glib};
use order::{Order};
use paths::THUMB_SUFFIX;
use clap::Parser;
use crate::entry::EntryList;
use crate::rank::Rank;

mod args;
mod direction;
mod entry;
mod gui;
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
        build_gui(&args, application);
    }));

    let empty: Vec<String> = vec![];
    application.run_with_args(&empty);
}



