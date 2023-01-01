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
    app.set_accels_for_action("action.next", &["n"]);
    app.run();
}

fn build_ui(app: &gtk::Application) {
    let selected_index = 0;
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("gallsh")
        .build();

    let action_next = SimpleAction::new_stateful(
        "next",
        Some(&u32::static_variant_type()),
        &selected_index.to_variant(),
        );
    action_next.connect_activate(clone!(@weak window => move |action,_| {
        let mut state = action
            .state()
            .expect("could not get state")
            .get::<u32>()
            .expect("the variant needs to be of type `u32`");
        state += 1;
        action.set_state(&state.to_variant());
        println!("selected index:{state}");
    }));

    let action_close = SimpleAction::new("close", None);
    action_close.connect_activate(clone!(@weak window => move |_, _| {
        window.close();
    }));
    window.add_action(&action_close);

    window.present();
}

