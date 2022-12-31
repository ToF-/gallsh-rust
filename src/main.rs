use std::env;
use gtk::prelude::*;
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

    window.present();
}
