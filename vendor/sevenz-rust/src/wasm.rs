use super::{password::Password, *};
use js_sys::*;
use std::io::{Read, Seek, SeekFrom, Write};
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn decompress(src: Uint8Array, pwd: &str, f: &Function) -> Result<(), String> {
    let mut src_reader = Uint8ArrayStream::new(src);
    let pos = src_reader.stream_position().map_err(|e| e.to_string())?;
    let len = src_reader
        .seek(SeekFrom::End(0))
        .map_err(|e| e.to_string())?;
    src_reader
        .seek(SeekFrom::Start(pos))
        .map_err(|e| e.to_string())?;
    let mut seven =
        SevenZReader::new(src_reader, len, Password::from(pwd)).map_err(|e| e.to_string())?;
    seven
        .for_each_entries(|entry, reader| {
            if !entry.is_directory() {
                let path = entry.name();

                if entry.size() > 0 {
                    let mut writer = Vec::new();
                    std::io::copy(reader, &mut writer).map_err(crate::Error::io)?;
                    f.call2(
                        &JsValue::NULL,
                        &JsValue::from(path),
                        &Uint8Array::from(&writer[..]),
                    );
                }
            }
            Ok(true)
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[wasm_bindgen]
pub fn compress(entries: Vec<JsString>, datas: Vec<Uint8Array>) -> Result<Uint8Array, String> {
    let output = Uint8Array::new_with_length(32);
    let writer = Uint8ArrayStream::new(output);

    let mut sz = SevenZWriter::new(writer).map_err(|e| e.to_string())?;
    let reader = SeqReader::new(
        datas
            .into_iter()
            .map(|d| Uint8ArrayStream::new(d))
            .map(SourceReader::new)
            .collect(),
    );
    let entries = entries
        .into_iter()
        .map(|name| SevenZArchiveEntry {
            name: name.into(),
            has_stream: true,
            ..Default::default()
        })
        .collect();

    sz.push_archive_entries(entries, reader)
        .map_err(|e| e.to_string())?;
    let out = sz.finish().map_err(|e| e.to_string())?;

    Ok(out.data)
}
struct Uint8ArrayStream {
    data: Uint8Array,
    pos: usize,
}

impl Seek for Uint8ArrayStream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(n) => {
                self.pos = n as usize;
            }
            SeekFrom::End(i) => {
                let posi = self.data.length() as i64 + i;
                if posi < 0 {
                    self.pos = 0;
                } else if posi >= self.data.length() as i64 {
                    self.pos = self.data.length() as usize;
                } else {
                    self.pos = posi as usize;
                }
            }
            SeekFrom::Current(i) => {
                if i != 0 {
                    let posi = self.pos as i64 + i;
                    if posi < 0 {
                        self.pos = 0;
                    } else if posi >= self.data.length() as i64 {
                        self.pos = self.data.length() as usize;
                    } else {
                        self.pos = posi as usize;
                    }
                }
            }
        }
        Ok(self.pos as u64)
    }
}

impl Uint8ArrayStream {
    fn new(data: Uint8Array) -> Self {
        Self { data, pos: 0 }
    }
}
impl Read for Uint8ArrayStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let end = (self.pos + buf.len()).min(self.data.length() as usize);
        let len = end - self.pos;
        if len == 0 {
            return Ok(0);
        }
        self.data
            .slice(self.pos as u32, end as u32)
            .copy_to(&mut buf[..len]);
        self.pos = end;
        Ok(len)
    }
}

impl Write for Uint8ArrayStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let end = (self.pos + buf.len()).min(self.data.length() as usize);
        let len = end - self.pos;
        if len == 0 {
            return Ok(0);
        }
        self.data
            .slice(self.pos as u32, end as u32)
            .copy_from(&buf[..len]);
        self.pos = end;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
