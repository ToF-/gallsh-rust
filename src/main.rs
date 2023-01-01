use std::env;
use gtk::prelude::*;
use gtk::gdk;
use gtk::glib;
use glib::clone;
use gtk::Application;
use gtk::gio;
use gio::SimpleAction;

fn main() {
    let mut gallshdir = String::from("images/");
    match env::var("GALLSHDIR")  {
        Ok(val) => gallshdir = String::from(val),
        Err(e) => {
            println!("GALLSHDIR: {e}\n default to \"{gallshdir}\"");
        }
    };
    println!("GALLSHDIR={gallshdir}");
    let app = Application::builder()
        .application_id("org.example.gallsh")
        .build();

    app.connect_activate(build_ui);
    app.set_accels_for_action("win.close", &["q"]);
    app.run();
}

fn build_ui(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("gallsh")
        .build();

    window
    .connect("key_press_event", false, move |values| {
        let raw_event = &values[1].get::<gdk::Event>().unwrap();
        match raw_event.downcast_ref::<gdk::EventKey>() {
            Some(event) => {
                println!("Key value: {:?}", event.keyval());
                println!("Modifier: {:?}", event.state());
                if event.keyval().to_unicode() == Some('q') {
                    println!("I should quit now");
                };
            },
            None => {},
        }

        let result = gtk::glib::value::Value::from_type(gtk::glib::types::Type::BOOL);
        Some(result)
    });

    let action_close = SimpleAction::new("close", None);
    action_close.connect_activate(clone!(@weak window => move |_, _| {
        window.close();
    }));
    window.add_action(&action_close);

    window.present();
}

