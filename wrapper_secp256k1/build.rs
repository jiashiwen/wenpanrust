use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=secp256k1");
    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from("bindings");
    bindings
        .write_to_file(out_path.join("secp256k1.rs"))
        .expect("Couldn't write bindings!");
}
