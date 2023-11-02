use std::{fs::File, io::Write, path::PathBuf, time::Instant};

fn main() {
    let mut sz = sevenz_rust::SevenZReader::open("examples/data/sample.7z", "pass".into()).unwrap();
    let total_size: u64 = sz
        .archive()
        .files
        .iter()
        .filter(|e| e.has_stream())
        .map(|e| e.size())
        .sum();
    let mut uncompressed_size = 0;
    let dest = PathBuf::from("examples/data/sample");
    sz.for_each_entries(|entry, reader| {
        let mut buf = [0u8; 1024];
        let path = dest.join(entry.name());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(path).unwrap();
        loop {
            let read_size = reader.read(&mut buf)?;
            if read_size == 0 {
                break Ok(true);
            }
            file.write_all(&buf[..read_size])?;
            uncompressed_size += read_size;
            println!("progress:{:.2}%", (uncompressed_size as f64 / total_size as f64)*100f64);
        }
    })
    .unwrap();
}
