LZMA/LZMA2 codec ported from [tukaani xz for java](https://tukaani.org/xz/java.html)


## Usage
### lzma 
```rust
    use std::io::{Read, Write};
    use lzma_rust::*;

    let s = b"Hello, world!";
    let mut out = Vec::new();
    let mut options = LZMA2Options::with_preset(6);
    options.dict_size = LZMA2Options::DICT_SIZE_DEFAULT;

    let mut w = LZMAWriter::new_use_header(CountingWriter::new(&mut out), &options, None).unwrap();
    w.write_all(s).unwrap();
    w.write(&[]).unwrap();
    let mut r = LZMAReader::new_mem_limit(&out[..], u32::MAX, None).unwrap();
    let mut s2 = vec![0; s.len()];
    r.read_exact(&mut s2).unwrap();
    println!("{:?}", &out[..]);
    assert_eq!(s, &s2[..]);

```

### lzma2
```rust
    use std::io::{Read, Write};
    use lzma_rust::*;

    let s = b"Hello, world!";
    let mut out = Vec::new();
    let mut options = LZMA2Options::with_preset(6);
    options.dict_size = LZMA2Options::DICT_SIZE_DEFAULT;
    {
        let mut w = LZMA2Writer::new(CountingWriter::new(&mut out), &options);
        w.write_all(s).unwrap();
        w.write(&[]).unwrap();
    }
    let mut r = LZMA2Reader::new(&out[..], options.dict_size, None);
    let mut s2 = vec![0; s.len()];
    r.read_exact(&mut s2).unwrap();
    println!("{:?}", &out[..]);
    assert_eq!(s, &s2[..]);

````