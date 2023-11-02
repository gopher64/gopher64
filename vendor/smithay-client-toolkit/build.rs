extern crate pkg_config;

fn main() {
    #[cfg(not(feature = "dlopen"))]
    pkg_config::Config::new().find("xkbcommon").unwrap();
}
