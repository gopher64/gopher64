use std::io::{Error, ErrorKind, Read, Result};

use byteorder::{LittleEndian, ReadBytesExt};

use super::decoder::LZMADecoder;
use super::lz::LZDecoder;
use super::range_dec::RangeDecoder;
use super::*;

pub fn get_memery_usage_by_props(dict_size: u32, props_byte: u8) -> Result<u32> {
    if dict_size > DICT_SIZE_MAX {
        return Err(Error::new(ErrorKind::InvalidInput, "dict size too large"));
    }
    if props_byte > (4 * 5 + 4) * 9 + 8 {
        return Err(Error::new(ErrorKind::InvalidInput, "Invalid props byte"));
    }
    let props = props_byte % (9 * 5);
    let lp = props / 9;
    let lc = props - lp * 9;
    get_memery_usage(dict_size, lc as u32, lp as u32)
}
pub fn get_memery_usage(dict_size: u32, lc: u32, lp: u32) -> Result<u32> {
    if lc > 8 || lp > 4 {
        return Err(Error::new(ErrorKind::InvalidInput, "Invalid lc or lp"));
    }
    return Ok(10 + get_dict_size(dict_size)? / 1024 + ((2 * 0x300) << (lc + lp)) / 1024);
}

fn get_dict_size(dict_size: u32) -> Result<u32> {
    if dict_size > DICT_SIZE_MAX {
        return Err(Error::new(ErrorKind::InvalidInput, "dict size too large"));
    }
    let dict_size = dict_size.max(4096);
    Ok((dict_size + 15) & !15)
}

/// # Examples
/// ```
/// use std::io::Read;
/// use lzma_rust::LZMAReader;
/// let compressed = [93, 0, 0, 128, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 36, 25, 73, 152, 111, 22, 2, 140, 232, 230, 91, 177, 71, 198, 206, 183, 99, 255, 255, 60, 172, 0, 0];
/// let mut reader = LZMAReader::new(&compressed[..]).unwrap();
/// let mut buf = [0; 1024];
/// let mut out = Vec::new();
/// loop {
///    let n = reader.read(&mut buf).unwrap();
///   if n == 0 {
///      break;
///   }
///   out.extend_from_slice(&buf[..n]);
/// }
/// assert_eq!(out, b"Hello, world!");
/// ```
pub struct LZMAReader<R> {
    lz: LZDecoder,
    rc: RangeDecoder<R>,
    lzma: LZMADecoder,
    end_reached: bool,
    relaxed_end_cond: bool,
    remaining_size: u64,
}

impl<R> Drop for LZMAReader<R> {
    fn drop(&mut self) {
        // self.reader.clone().release();
    }
}

impl<R: Read> LZMAReader<R> {
    fn construct1(
        reader: R,
        uncomp_size: u64,
        mut props: u8,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        if props > (4 * 5 + 4) * 9 + 8 {
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid props byte"));
        }
        let pb = props / (9 * 5);
        props -= pb * 9 * 5;
        let lp = props / 9;
        let lc = props - lp * 9;
        if dict_size > DICT_SIZE_MAX {
            return Err(Error::new(ErrorKind::InvalidInput, "dict size too large"));
        }
        Self::construct2(
            reader,
            uncomp_size,
            lc as _,
            lp as _,
            pb as _,
            dict_size,
            preset_dict,
        )
    }

    fn construct2(
        reader: R,
        uncomp_size: u64,
        lc: u32,
        lp: u32,
        pb: u32,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        if lc > 8 || lp > 4 || pb > 4 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Invalid lc or lp or pb",
            ));
        }
        let mut dict_size = get_dict_size(dict_size)?;
        if uncomp_size <= u64::MAX / 2 && dict_size as u64 > uncomp_size {
            dict_size = get_dict_size(uncomp_size as u32)?;
        }
        let rc = RangeDecoder::new_stream(reader);
        let rc = match rc {
            Ok(r) => r,
            Err(e) => {
                return Err(e);
            }
        };
        let lz = LZDecoder::new(get_dict_size(dict_size)? as _, preset_dict);
        let lzma = LZMADecoder::new(lc, lp, pb);
        Ok(Self {
            // reader,
            lz,
            rc,
            lzma,
            end_reached: false,
            relaxed_end_cond: true,
            remaining_size: uncomp_size,
        })
    }

    ///
    /// Creates a new .lzma file format decompressor with an optional memory usage limit.
    /// - [mem_limit_kb] - memory usage limit in kibibytes (KiB). u32::MAX means no limit.
    /// - [preset_dict] - preset dictionary or None to use no preset dictionary.
    pub fn new_mem_limit(
        mut reader: R,
        mem_limit_kb: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        let props = reader.read_u8()?;
        let dict_size = reader.read_u32::<LittleEndian>()?;

        let uncomp_size = reader.read_u64::<LittleEndian>()?;
        let need_mem = get_memery_usage_by_props(dict_size, props)?;
        if mem_limit_kb < need_mem {
            return Err(Error::new(
                ErrorKind::OutOfMemory,
                format!(
                    "{}kb memery needed,but limit was {}kb",
                    need_mem, mem_limit_kb
                ),
            ));
        }
        Self::construct1(reader, uncomp_size, props, dict_size, preset_dict)
    }

    /// Creates a new input stream that decompresses raw LZMA data (no .lzma header) from `reader` optionally with a preset dictionary.
    /// - [reader] - the reader to read compressed data from.
    /// - [uncomp_size] - the uncompressed size of the data to be decompressed.
    /// - [props] - the LZMA properties byte.
    /// - [dict_size] - the LZMA dictionary size.
    /// - [preset_dict] - preset dictionary or None to use no preset dictionary.
    pub fn new_with_props(
        reader: R,
        uncomp_size: u64,
        props: u8,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        Self::construct1(reader, uncomp_size, props, dict_size, preset_dict)
    }

    /// Creates a new input stream that decompresses raw LZMA data (no .lzma header) from `reader` optionally with a preset dictionary.
    /// - [reader] - the input stream to read compressed data from.
    /// - [uncomp_size] - the uncompressed size of the data to be decompressed.
    /// - [lc] - the number of literal context bits.
    /// - [lp] - the number of literal position bits.
    /// - [pb] - the number of position bits.
    /// - [dict_size] - the LZMA dictionary size.
    /// - [preset_dict] - preset dictionary or None to use no preset dictionary.
    pub fn new(
        reader: R,
        uncomp_size: u64,
        lc: u32,
        lp: u32,
        pb: u32,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        Self::construct2(reader, uncomp_size, lc, lp, pb, dict_size, preset_dict)
    }

    fn read_decode(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.end_reached {
            return Ok(0);
        }
        let mut size = 0;
        let mut len = buf.len() as u32;
        let mut off = 0u32;
        while len > 0 {
            let mut copy_size_max = len as u32;
            if self.remaining_size <= u64::MAX / 2 && (self.remaining_size as u32) < len {
                copy_size_max = self.remaining_size as u32;
            }
            self.lz.set_limit(copy_size_max as usize);

            match self.lzma.decode(&mut self.lz, &mut self.rc) {
                Ok(_) => {}
                Err(e) => {
                    if self.remaining_size != u64::MAX || !self.lzma.end_marker_detected() {
                        return Err(e);
                    }
                    self.end_reached = true;
                    self.rc.normalize()?;
                }
            }

            let copied_size = self.lz.flush(buf, off as _) as u32;
            off += copied_size;
            len -= copied_size;
            size += copied_size;
            if self.remaining_size <= u64::MAX / 2 {
                self.remaining_size -= copied_size as u64;
                if self.remaining_size == 0 {
                    self.end_reached = true;
                }
            }

            if self.end_reached {
                if self.lz.has_pending()
                    || (!self.relaxed_end_cond && !self.rc.is_stream_finished())
                {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "end reached but not decoder finished",
                    ));
                }
                return Ok(size as _);
            }
        }
        Ok(size as _)
    }
}

impl<R: Read> Read for LZMAReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.read_decode(buf)
    }
}
