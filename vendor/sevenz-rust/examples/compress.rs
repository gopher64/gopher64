use std::time::Instant;

fn main() {
    let now = Instant::now();
    #[cfg(all(feature = "compress", feature = "aes"))]
    sevenz_rust::compress_to_path_encrypted(
        "examples/data/sample",
        "examples/data/sample.7z",
        "pass".into(),
    )
    .expect("compress ok");
    println!("compress done : {:?}", now.elapsed());
}
