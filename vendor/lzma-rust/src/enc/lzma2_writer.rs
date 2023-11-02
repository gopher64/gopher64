use std::io::{ErrorKind, Write};

use byteorder::WriteBytesExt;

use super::counting::CountingWriter;

use super::encoder::LZMAEncoderModes;
use super::{
    encoder::{EncodeMode, LZMAEncoder},
    lz::MFType,
    range_enc::{RangeEncoder, RangeEncoderBuffer},
};

#[derive(Debug, Clone)]
pub struct LZMA2Options {
    pub dict_size: u32,
    pub lc: u32,
    pub lp: u32,
    pub pb: u32,
    pub mode: EncodeMode,
    pub nice_len: u32,
    pub mf: MFType,
    pub depth_limit: i32,
    pub preset_dict: Option<Vec<u8>>,
}

impl Default for LZMA2Options {
    fn default() -> Self {
        Self::with_preset(6)
    }
}
impl LZMA2Options {
    pub const LC_DEFAULT: u32 = 3;
    pub const LP_DEFAULT: u32 = 0;
    pub const PB_DEFAULT: u32 = 2;
    pub const NICE_LEN_MAX: u32 = 273;
    pub const NICE_LEN_MIN: u32 = 8;
    pub const DICT_SIZE_DEFAULT: u32 = 8 << 20;
    const PRESET_TO_DICT_SIZE: &'static [u32] = &[
        1 << 18,
        1 << 20,
        1 << 21,
        1 << 22,
        1 << 22,
        1 << 23,
        1 << 23,
        1 << 24,
        1 << 25,
        1 << 26,
    ];
    const PRESET_TO_DEPTH_LIMIT: &'static [i32] = &[4, 8, 24, 48];
    pub fn new(
        dict_size: u32,
        lc: u32,
        lp: u32,
        pb: u32,
        mode: EncodeMode,
        nice_len: u32,
        mf: MFType,
        depth_limit: i32,
    ) -> Self {
        Self {
            dict_size,
            lc,
            lp,
            pb,
            mode,
            nice_len,
            mf,
            depth_limit,
            preset_dict: None,
        }
    }

    /// preset: [0..9]
    #[inline]
    pub fn with_preset(preset: u32) -> Self {
        let mut opt = Self {
            dict_size: Default::default(),
            lc: Default::default(),
            lp: Default::default(),
            pb: Default::default(),
            mode: EncodeMode::Normal,
            nice_len: Default::default(),
            mf: Default::default(),
            depth_limit: Default::default(),
            preset_dict: Default::default(),
        };
        opt.set_preset(preset);
        opt
    }

    /// preset: [0..9]
    pub fn set_preset(&mut self, preset: u32) {
        if preset > 9 {
            return;
        }
        self.lc = Self::LC_DEFAULT;
        self.lp = Self::LP_DEFAULT;
        self.pb = Self::PB_DEFAULT;
        self.dict_size = Self::PRESET_TO_DICT_SIZE[preset as usize];
        if preset <= 3 {
            self.mode = EncodeMode::Fast;
            self.mf = MFType::HC4;
            self.nice_len = if preset <= 1 { 128 } else { Self::NICE_LEN_MAX };
            self.depth_limit = Self::PRESET_TO_DEPTH_LIMIT[preset as usize];
        } else {
            self.mode = EncodeMode::Normal;
            self.mf = MFType::BT4;
            self.nice_len = if preset == 4 {
                16
            } else if preset == 5 {
                32
            } else {
                64
            };
            self.depth_limit = 0;
        }
    }

    pub fn get_memery_usage(&self) -> u32 {
        let dict_size = self.dict_size;
        let extra_size_before = get_extra_size_before(dict_size);
        70 + LZMAEncoder::get_mem_usage(self.mode, dict_size, extra_size_before, self.mf)
    }

    #[inline(always)]
    pub fn get_props(&self) -> u8 {
        ((self.pb * 5 + self.lp) * 9 + self.lc) as u8
    }
}
const COMPRESSED_SIZE_MAX: u32 = 64 << 10;
pub fn get_extra_size_before(dict_size: u32) -> u32 {
    return if COMPRESSED_SIZE_MAX > dict_size {
        COMPRESSED_SIZE_MAX - dict_size
    } else {
        0
    };
}

/// LZMA2 format writer
/// # Examples
/// ```
/// use std::io::Write;
/// use lzma_rust::enc::lzma2_writer::{LZMA2Options, LZMA2Writer};
/// let mut writer = LZMA2Writer::new(Vec::new(), &LZMA2Options::default());
///    writer.write_all(b"hello world").unwrap();
///    let compressed = writer.finish().unwrap();
///
/// ```
pub struct LZMA2Writer<W: Write> {
    inner: CountingWriter<W>,
    rc: RangeEncoder<RangeEncoderBuffer>,
    lzma: LZMAEncoder,
    mode: LZMAEncoderModes,
    props: u8,
    dict_reset_needed: bool,
    state_reset_needed: bool,
    props_needed: bool,
    pending_size: u32,
    finished: bool,
}

impl<W: Write> LZMA2Writer<W> {
    pub fn new(inner: CountingWriter<W>, options: &LZMA2Options) -> Self {
        let dict_size = options.dict_size;
        let rc = RangeEncoder::new_buffer(COMPRESSED_SIZE_MAX as usize);
        let (mut lzma, mode) = LZMAEncoder::new(
            options.mode,
            options.lc,
            options.lp,
            options.pb,
            options.mf,
            options.depth_limit,
            options.dict_size,
            options.nice_len as usize,
        );

        let props = options.get_props();
        let mut dict_reset_needed = true;
        if let Some(preset_dict) = &options.preset_dict {
            lzma.lz.set_preset_dict(dict_size, preset_dict);
            dict_reset_needed = false;
        }
        Self {
            inner,
            rc,
            lzma,
            mode,
            props,
            dict_reset_needed,
            state_reset_needed: true,
            props_needed: true,
            pending_size: 0,
            finished: false,
        }
    }

    fn write_lzma(&mut self, uncompressed_size: u32, compressed_size: u32) -> std::io::Result<()> {
        let mut control = if self.props_needed {
            if self.dict_reset_needed {
                0x80 + (3 << 5)
            } else {
                0x80 + (2 << 5)
            }
        } else {
            if self.state_reset_needed {
                0x80 + (1 << 5)
            } else {
                0x80
            }
        };
        control = control | (uncompressed_size - 1) >> 16;
        let mut chunk_header = [0u8; 6];
        chunk_header[0] = control as u8;
        chunk_header[1] = ((uncompressed_size - 1) >> 8) as u8;
        chunk_header[2] = (uncompressed_size - 1) as u8;
        chunk_header[3] = ((compressed_size - 1) >> 8) as u8;
        chunk_header[4] = (compressed_size - 1) as u8;
        if self.props_needed {
            chunk_header[5] = self.props as u8;
            self.inner.write_all(&chunk_header)?;
        } else {
            self.inner.write_all(&chunk_header[..5])?;
        }

        self.rc.write_to(&mut self.inner)?;
        self.props_needed = false;
        self.state_reset_needed = false;
        self.dict_reset_needed = false;
        Ok(())
    }

    fn write_uncompressed(&mut self, mut uncompressed_size: u32) -> std::io::Result<()> {
        while uncompressed_size > 0 {
            let chunk_size = uncompressed_size.min(COMPRESSED_SIZE_MAX as u32);
            let mut chunk_header = [0u8; 3];
            chunk_header[0] = if self.dict_reset_needed { 0x01 } else { 0x02 };
            chunk_header[1] = ((chunk_size - 1) >> 8) as u8;
            chunk_header[2] = (chunk_size - 1) as u8;
            self.inner.write_all(&chunk_header)?;
            self.lzma.lz.copy_uncompressed(
                &mut self.inner,
                uncompressed_size as i32,
                chunk_size as usize,
            )?;
            uncompressed_size -= chunk_size;
            self.dict_reset_needed = false;
        }
        self.state_reset_needed = true;
        Ok(())
    }
    fn write_chunk(&mut self) -> std::io::Result<()> {
        let compressed_size = self.rc.finish_buffer()?.unwrap_or_default() as u32;
        let mut uncompressed_size = self.lzma.data.uncompressed_size;
        assert!(compressed_size > 0);
        assert!(
            uncompressed_size > 0,
            "uncompressed_size is 0, read_pos={}",
            self.lzma.lz.read_pos
        );
        if compressed_size + 2 < uncompressed_size {
            self.write_lzma(uncompressed_size, compressed_size)?;
        } else {
            self.lzma.reset(&mut self.mode);
            uncompressed_size = self.lzma.data.uncompressed_size;
            assert!(uncompressed_size > 0);
            self.write_uncompressed(uncompressed_size)?;
        }
        self.pending_size -= uncompressed_size;
        self.lzma.reset_uncompressed_size();
        self.rc.reset_buffer();
        Ok(())
    }
    fn write_end_marker(&mut self) -> std::io::Result<()> {
        assert!(!self.finished);

        self.lzma.lz.set_finishing();

        while self.pending_size > 0 {
            self.lzma.encode_for_lzma2(&mut self.rc, &mut self.mode)?;
            self.write_chunk()?;
        }

        self.inner.write_u8(0x00)?;
        self.finished = true;

        Ok(())
    }

    pub fn finish(&mut self) -> std::io::Result<()> {
        if !self.finished {
            self.write_end_marker()?;
        }
        Ok(())
    }
}

impl<W: Write> Drop for LZMA2Writer<W> {
    fn drop(&mut self) {}
}
impl<W: Write> Write for LZMA2Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut len = buf.len();
        if len == 0 && !self.finished {
            self.finish()?;
            self.inner.write(buf)?;
            return Ok(0);
        }
        if self.finished {
            return Err(std::io::Error::new(ErrorKind::Other, "LZMA2 finished"));
        }

        let mut off = 0;
        while len > 0 {
            let used = self.lzma.lz.fill_window(&buf[off..(off + len)]);
            off += used;
            len -= used;
            self.pending_size += used as u32;
            if self.lzma.encode_for_lzma2(&mut self.rc, &mut self.mode)? {
                self.write_chunk()?;
            }
        }
        Ok(off)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.finished {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "LZMA2 flush finished",
            ));
        }
        self.lzma.lz.set_flushing();
        while self.pending_size > 0 {
            self.lzma.encode_for_lzma2(&mut self.rc, &mut self.mode)?;
            self.write_chunk()?;
        }
        self.inner.flush()
    }
}
