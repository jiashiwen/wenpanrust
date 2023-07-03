use std::path::PathBuf;

fn main() {
    // cc::Build::new().file("sample.c").compile("sample");

    // 参考cc 文档
    println!("cargo:rerun-if-changed=sample.c");
    cc::Build::new()
        .file("sample.c")
        .shared_flag(true)
        .compile("sample.so");
    // 参考 https://doc.rust-lang.org/cargo/reference/build-scripts.html
    println!("cargo:rustc-link-lib=sample.so");
    println!("cargo:rerun-if-changed=sample.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // .header("sample.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = PathBuf::from("bindings");
    bindings
        .write_to_file(out_path.join("sample_bindings.rs"))
        .expect("Couldn't write bindings!");
}
