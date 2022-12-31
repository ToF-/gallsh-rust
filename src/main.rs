use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use std::env;

const  APP_ID: &str = "org.gtk_rs.gallsh";

fn main() {
    let mut gallshdir = String::from("images/");
    match env::var("GALLSHDIR")  {
        Ok(val) => gallshdir = String::from(val),
        Err(e) => {
            println!("GALLSHDIR: {e}\n default to \"{gallshdir}\"");
        }
    }
    println!("GALLSHDIR={gallshdir}");
    
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("gallsh")
        .build();
    window.present();
}
