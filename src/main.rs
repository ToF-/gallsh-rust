use std::env;

fn main() {
    let mut gallshdir = String::from("images/");
    match env::var("GALLSHDIR")  {
        Ok(val) => gallshdir = String::from(val),
        Err(e) => {
            println!("GALLSHDIR: {e}\n default to \"{gallshdir}\"");
        }
    }
    println!("GALLSHDIR={gallshdir}");
}
