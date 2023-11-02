use std::path::PathBuf;

use sevenz_rust::{Archive, FolderDecoder, Password};

fn main() {
    let mut file = std::fs::File::open("examples/data/sample.7z").unwrap();
    let len = file.metadata().unwrap().len();
    let password = Password::empty();
    let archive = Archive::read(&mut file, len, password.as_slice()).unwrap();
    let folder_count = archive.folders.len();
    let my_file_name = "7zFormat.txt";

    for folder_index in 0..folder_count {
        let forder_dec = FolderDecoder::new(folder_index, &archive, password.as_slice(), &mut file);

        if forder_dec
            .entries()
            .iter()
            .find(|entry| entry.name() == my_file_name)
            .is_none()
        {
            // skip the folder if it does not contain the file we want
            continue;
        }
        let dest = PathBuf::from("examples/data/sample_mt/");

        forder_dec
            .for_each_entries(&mut |entry, reader| {
                if entry.name() == my_file_name {
                    //only extract the file we want
                    let dest = dest.join(entry.name());
                    sevenz_rust::default_entry_extract_fn(entry, reader, &dest)?;
                } else {
                    //skip other files
                    std::io::copy(reader, &mut std::io::sink())?;
                }
                Ok(true)
            })
            .expect("ok");
    }
}
