use std::{
    fs::{read, read_to_string, File},
    path::PathBuf,
};

use tempfile::tempdir;

use sevenz_rust::{decompress_file, Archive, FolderDecoder};

#[test]
fn decompress_single_empty_file_unencoded_header() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/single_empty_file.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("empty.txt");

    decompress_file(source_file, target).unwrap();

    assert_eq!(read_to_string(file1_path).unwrap(), "");
}

#[test]
fn decompress_two_empty_files_unencoded_header() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/two_empty_file.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("file1.txt");
    let mut file2_path = target.clone();
    file2_path.push("file2.txt");

    decompress_file(source_file, target).unwrap();

    assert_eq!(read_to_string(file1_path).unwrap(), "");
    assert_eq!(read_to_string(file2_path).unwrap(), "");
}

#[test]
fn decompress_lzma_single_file_unencoded_header() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/single_file_with_content_lzma.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("file.txt");

    decompress_file(source_file, target).unwrap();

    assert_eq!(read_to_string(file1_path).unwrap(), "this is a file\n");
}

#[test]
fn decompress_lzma2_bcj_x86_file() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/decompress_example_lzma2_bcj_x86.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("decompress.exe");

    decompress_file(source_file, target).unwrap();

    let mut expected_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    expected_file.push("tests/resources/decompress_x86.exe");

    assert!(
        read(file1_path).unwrap() == read(expected_file).unwrap(),
        "decompressed files do not match!"
    );
}

#[test]
fn decompress_lzma_multiple_files_encoded_header() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/two_files_with_content_lzma.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("file1.txt");
    let mut file2_path = target.clone();
    file2_path.push("file2.txt");

    decompress_file(source_file, target).unwrap();

    assert_eq!(read_to_string(file1_path).unwrap(), "file one content\n");
    assert_eq!(read_to_string(file2_path).unwrap(), "file two content\n");
}

#[test]
fn decompress_delta_lzma_single_file_unencoded_header() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/delta.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("delta.txt");

    decompress_file(source_file, target).unwrap();

    assert_eq!(read_to_string(file1_path).unwrap(), "aaaabbbbcccc");
}

#[test]
fn decompress_copy_lzma2_single_file() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/copy.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();
    let mut file1_path = target.clone();
    file1_path.push("copy.txt");

    decompress_file(source_file, target).unwrap();

    assert_eq!(read_to_string(file1_path).unwrap(), "simple copy encoding");
}

#[cfg(feature = "bzip2")]
#[test]
fn decompress_bzip2_file() {
    let mut source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_file.push("tests/resources/bzip2_file.7z");
    let temp_dir = tempdir().unwrap();
    let target = temp_dir.path().to_path_buf();

    let mut hello_path = target.clone();
    hello_path.push("hello.txt");

    let mut foo_path = target.clone();
    foo_path.push("foo.txt");

    decompress_file(source_file, target).unwrap();

    assert_eq!(read_to_string(hello_path).unwrap(), "world\n");
    assert_eq!(read_to_string(foo_path).unwrap(), "bar\n");
}

#[test]
fn test_bcj2() {
    let mut file = File::open("tests/resources/7za433_7zip_lzma2_bcj2.7z").unwrap();
    let file_len = file.metadata().unwrap().len();
    let archive = Archive::read(&mut file, file_len, &[]).unwrap();
    for i in 0..archive.folders.len() {
        let fd = FolderDecoder::new(i, &archive, &[], &mut file);
        println!("entry_count:{}", fd.entry_count());
        fd.for_each_entries(&mut |entry, reader| {
            println!("{}=>{:?}", entry.has_stream, entry.name());
            std::io::copy(reader, &mut std::io::sink())?;
            Ok(true)
        })
        .unwrap();
    }
}
