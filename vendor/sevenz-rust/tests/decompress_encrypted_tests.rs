use sevenz_rust::*;
use std::{fs::read_to_string, path::PathBuf};
use tempfile::tempdir;

#[cfg(feature = "aes256")]
#[test]
fn test_decompress_file_with_password() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/encrypted.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("encripted/7zFormat.txt");
    let r = decompress_file_with_password(source_file, target.as_path(), "sevenz-rust".into());
    assert!(r.is_ok());
    assert!(read_to_string(file1_path)
        .unwrap()
        .starts_with("7z is the new archive format, providing high compression ratio."))
}
