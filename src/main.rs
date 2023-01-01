use std::env;
use gtk::prelude::*;
use gtk::gdk;
use gtk::glib;
use gtk::Application;
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
    app.run();
}

fn build_ui(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("gallsh")
        .build();

    window
    .connect("key_press_event", false, |values| {
        let raw_event = &values[1].get::<gdk::Event>().unwrap();
        match raw_event.downcast_ref::<gdk::EventKey>() {
            Some(event) => {
                println!("Key value: {:?}", event.keyval());
                println!("Modifier: {:?}", event.state());
            },
            None => {},
        }

        let result = glib::value::Value::from_type(glib::types::Type::BOOL);
        Some(result)
    });

    window.present();
}
