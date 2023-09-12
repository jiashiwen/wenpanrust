fn main() {
    println!("Hello, world!");
    print_os();

    #[cfg(feature = "feature_1")]
    print_feature();

    #[cfg(all(target_os = "linux", feature = "feature_2"))]
    print_feature_macos();
}

#[cfg(target_os = "macos")]
fn print_os() {
    println!("os is:{}", "mcaos")
}

#[cfg(target_os = "linux")]
fn print_os() {
    println!("os is:{}", "linux")
}

#[cfg(feature = "feature_1")]
fn print_feature() {
    println!("feature one")
}

// cargo run --features feature_2
// #[cfg(all(target_os = "macos", feature = "feature_2"))]
#[cfg_attr(
    target_os = "linux",
    cfg_attr(feature = "feature_2", some_other_attribute)
)]
fn print_feature_macos() {
    println!("macos feature tow")
}
