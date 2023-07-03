// extern "C" {
//     #[link_name = "abs"]
//     fn abs_in_rust(input: i32) -> i32;
// }

// fn abs_sample() {
//     unsafe {
//         println!("abs(-123) is {}", abs_in_rust(-123));
//     }
// }

// fn main() {
//     println!("Hello, world!");
//     abs_sample();
// }

// use std::os::raw::c_int;
// include!(concat!(env!("OUT_DIR"), "/sample_bindings.rs"));
include!("../bindings/sample_bindings.rs");

// #[link(name = "sample")]
// extern "C" {
//     fn add(a: c_int, b: c_int) -> c_int;
// }

fn main() {
    let r = unsafe { add(2, 18) };
    println!("{:?}", r);
}
