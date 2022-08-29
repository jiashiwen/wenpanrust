use crate::resources::get_app_default;
use std::fs;
use std::path::Path;

mod resources;

fn main() {
    if Path::new("./app.yml").exists() {
        let contents =
            fs::read_to_string("./app.yml").expect("Read file error!");
        println!("{}", contents);
        return;
    }
    let app = get_app_default().unwrap();
    let f = std::str::from_utf8(app.data.as_ref()).unwrap();
    println!("{}", f);
}
