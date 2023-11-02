fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS") == Ok("macos".to_string()) {
        println!("cargo:rustc-link-lib=framework=CoreHaptics");
    }
}