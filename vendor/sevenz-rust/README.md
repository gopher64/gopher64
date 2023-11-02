[![Crate](https://img.shields.io/crates/v/sevenz-rust.svg)](https://crates.io/crates/sevenz-rust)
 [![Documentation](https://docs.rs/sevenz-rust/badge.svg)](https://docs.rs/sevenz-rust)
 
This project is a 7z compressor/decompressor written in pure rust.<br/>
And it's very much inspired by the [apache commons-compress](https://commons.apache.org/proper/commons-compress/) project.<br/>

The LZMA/LZMA2 decoder and all filters code was ported from [tukaani xz for java](https://tukaani.org/xz/java.html)

## Decompression

Supported codecs:
 - [x] BZIP2 (require feature 'bzip2')
 - [x] COPY
 - [x] LZMA
 - [x] LZMA2
 - [x] ZSTD  (require feature 'zstd')


Supported filters:
 - [x] BCJ X86
 - [x] BCJ PPC
 - [x] BCJ IA64
 - [x] BCJ ARM
 - [x] BCJ ARM_THUMB
 - [x] BCJ SPARC
 - [x] DELTA
 - [x] BJC2




### Usage

```
[dependencies]
sevenz-rust={version="0.2"}
```

Decompress source file "data/sample.7z" to dest path "data/sample"
```rust
sevenz_rust::decompress_file("data/sample.7z", "data/sample").expect("complete");
```

#### Decompress a encrypted 7z file

Add 'aes256' feature
```
[dependencies]
sevenz-rust={version="0.2", features=["aes256"]}
```

```rust
sevenz_rust::decompress_file_with_password("path/to/encrypted.7z", "path/to/output", "password".into()).expect("complete");
```

#### Multi-thread decompress
check [examples/mt_decompress](https://github.com/dyz1990/sevenz-rust/blob/main/examples/mt_decompress.rs)



## Compression
Currently only support LZMA2 method.

```
[dependencies]
sevenz-rust={version="0.5.0", features=["compress"]}
```

Use the helper function to create a 7z file with source path.
```rust
sevenz_rust::compress_to_path("examples/data/sample", "examples/data/sample.7z").expect("compress ok");
```

### With AES encryption
require version>=0.3.0
```
[dependencies]
sevenz-rust={version="0.5", features=["compress","aes256"]}
```

Use the helper function to create a 7z file with source path and password.
```rust
sevenz_rust::compress_to_path_encrypted("examples/data/sample", "examples/data/sample.7z", "password".into()).expect("compress ok");
```

### Advance

```
[dependencies]
sevenz-rust={version="0.5.0", features=["compress","aes256"]}
```

#### Solid compression

```
use sevenz_rust::*;

let mut sz = SevenZWriter::create("dest.7z").expect("create writer ok");

sz.push_source_path("path/to/compress", |_| true).expect("pack ok");

sz.finish().expect("compress ok");

```


#### Compression methods

with encryption and lzma2 options

```
use sevenz_rust::*;

let mut sz = SevenZWriter::create("dest.7z").expect("create writer ok");
sz.set_content_methods(vec![
    sevenz_rust::AesEncoderOptions::new("sevenz-rust".into()).into(),
    lzma::LZMA2Options::with_preset(9).into(),
]);
sz.push_source_path("path/to/compress", |_| true).expect("pack ok");

sz.finish().expect("compress ok");

```

## Changelog
### 0.5.3
Fixed 'Too many open files'
Reduce unnecessary public items #37

### 0.5.2 - 2023-0824
Fixed file separator issue on windows system #35

### 0.5.1 - 2023-0823
Sub crate `lzma-rust` code optimization

### 0.5.0 - 2023-0819
- Added support for BCJ2.
- Added multi-thread decompress example

### 0.4.3 - 2023-0616
- Support write encoded header
- Added `LZMAWriter`
### 0.4.2 - 2023-06-10
- Removed unsafe code
- Changed `SevenZWriter.finish` method return inner writer
- Added wasm compress function
- Updates bzip dependency to the patch version of 0.4.4([#23](https://github.com/dyz1990/sevenz-rust/pull/23))

### 0.4.1 - 2023-06-07
- Fixed unable to build without default features

### 0.4.0 - 2023-06-03 - Solid compression

### 0.3.0 - 2023-06-02 - Encrypted compression
- Added Encrypted compression
### 0.2.11 - 2023-05-24
- Fixed numerical overflow
### 0.2.10 - 2023-04-18
- Change to use nt-time crate([#20](https://github.com/dyz1990/sevenz-rust/pull/20))
- Fix typo([#18](https://github.com/dyz1990/sevenz-rust/pull/18))
- make function generics less restrictive ([#17](https://github.com/dyz1990/sevenz-rust/pull/17))
- Solve warnings ([#16](https://github.com/dyz1990/sevenz-rust/pull/16))
- run rustfmt on code ([#15](https://github.com/dyz1990/sevenz-rust/pull/15))

### 0.2.9 - 2023-03-16
- Added bzip2 support([#14](https://github.com/dyz1990/sevenz-rust/pull/14))

### 0.2.8 - 2023-03-06
- Fixed write bitset bugs
### 0.2.7 - 2023-03-05
- Fixed bug while read files info

### 0.2.6 - 2023-02-23
- Added zstd support and use enhanced filetime lib([#11](https://github.com/dyz1990/sevenz-rust/pull/11))
- Fixed lzma encoder bugs
  
### 0.2.4 - 2023-02-16
- Changed return entry ref when pushing to writer([#10](https://github.com/dyz1990/sevenz-rust/pull/10))

### 0.2.3 - 2023-02-07
- Fixed incorrect handling of 7z time

### 0.2.2 - 2023-01-31 - Create sub crate `lzma-rust`
- Move mod `lzma` to sub crate `lzma-rust`
- Modify Github Actions to run tests with --all-features

### 0.2.0 - 2023-01-08 - Added compression supporting
- Added compression supporting

### 0.1.5 - 2022-11-01 - Encrypted 7z files decompression supported
- Added aes256sha256 decode method
- Added wasm support
- Added new tests (for Delta and Copy) and Github Actions CI([#5](https://github.com/dyz1990/sevenz-rust/pull/5)) by [bfrazho](https://github.com/bfrazho)

### 0.1.4 - 2022-09-20 - Replace lzma/lzma2 decoder
- Chnaged new lzma/lzma2 decoder


### 0.1.3 - 2022-09-18 - add more bcj filters

- Added bcj arm/ppc/sparc and delta filters
- Added test for bcj x86 ([#3](https://github.com/dyz1990/sevenz-rust/pull/3)) by [bfrazho](https://github.com/bfrazho)

### 0.1.2 - 2022-09-14 - bcj x86 filter supported
- Added bcj x86 filter 
- Added LZMA tests ([#2](https://github.com/dyz1990/sevenz-rust/pull/2)) by [bfrazho](https://github.com/bfrazho)
- Fixed extract empty file

### 0.1.1 - 2022-08-10 - Modify decompression function

### 0.1.0 - 2022-08-10 - Decompression
