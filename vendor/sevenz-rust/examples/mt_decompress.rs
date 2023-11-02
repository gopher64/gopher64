use std::{path::PathBuf, sync::Arc};

use sevenz_rust::{Archive, FolderDecoder, Password};

fn main() {
    let time = std::time::Instant::now();
    let mut file = std::fs::File::open("examples/data/sample.7z").unwrap();
    let len = file.metadata().unwrap().len();
    let password = Password::empty();
    let archive = Archive::read(&mut file, len, password.as_slice()).unwrap();
    let folder_count = archive.folders.len();
    if folder_count <= 1 {
        println!("folder count less than 1, use single thread");
        //TODO use single thread
    }
    let archive = Arc::new(archive);
    let password = Arc::new(password);

    let mut threads = Vec::new();
    for folder_index in 0..folder_count {
        let archive = archive.clone();
        let password = password.clone();
        //TODO: use thread pool
        let handle = std::thread::spawn(move || {
            let mut source = std::fs::File::open("examples/data/sample.7z").unwrap();
            let forder_dec =
                FolderDecoder::new(folder_index, &archive, password.as_slice(), &mut source);
            let dest = PathBuf::from("examples/data/sample_mt/");
            forder_dec
                .for_each_entries(&mut |entry, reader| {
                    let dest = dest.join(entry.name());
                    sevenz_rust::default_entry_extract_fn(entry, reader, &dest)?;
                    Ok(true)
                })
                .expect("ok");
        });
        threads.push(handle);
    }

    threads
        .into_iter()
        .for_each(|handle| handle.join().unwrap());
    println!("multi-thread decompress use time:{:?}", time.elapsed());
}
