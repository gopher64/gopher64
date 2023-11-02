#![feature(str_checked_slicing)]

use std::fs::File;
use std::io::{BufRead, BufReader, Write};

fn main() {
    let input = File::open("unifont.hex").unwrap();
    let mut output = File::create("unifont.font").unwrap();

    let mut count = 0;
    for line_res in BufReader::new(input).lines() {
        let line = line_res.unwrap();

        let mut parts = line.split(":");
        let num = u32::from_str_radix(parts.next().unwrap(), 16).unwrap();

        while count < num {
            output.write(&[0; 16]).unwrap();
            count += 1;
        }

        assert_eq!(num, count);

        let mut data = [0; 16];
        let data_part = parts.next().unwrap();
        for i in 0..data.len() {
            let string = data_part.get(i * 2 .. i * 2 + 2).unwrap_or("00");
            data[i] = u8::from_str_radix(string, 16).unwrap();
        }
        println!("{:>04X}:{:?}", num, data);

        output.write(&data).unwrap();
        count += 1;
    }
}
