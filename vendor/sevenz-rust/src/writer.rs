mod pack_info;
mod seq_reader;
mod unpack_info;

use crate::{archive::*, encoders, lzma::*, reader::CRC32, Error, SevenZArchiveEntry};
use bit_set::BitSet;
use byteorder::*;
use std::{
    cell::Cell,
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
    rc::Rc,
    sync::Arc,
};

pub use self::seq_reader::*;
use self::{pack_info::PackInfo, unpack_info::UnpackInfo};

macro_rules! write_times {
    //write_i64
    ($fn_name:tt, $nid:expr, $has_time:tt, $time:tt) => {
        write_times!($fn_name, $nid, $has_time, $time, write_u64);
    };
    ($fn_name:tt, $nid:expr, $has_time:tt, $time:tt, $write_fn:tt) => {
        fn $fn_name<H: Write>(&self, header: &mut H) -> std::io::Result<()> {
            let mut num = 0;
            for entry in self.files.iter() {
                if entry.$has_time {
                    num += 1;
                }
            }
            if num > 0 {
                header.write_u8($nid)?;
                let mut temp: Vec<u8> = Vec::with_capacity(128);
                let mut out = &mut temp;
                if num != self.files.len() {
                    out.write_u8(0)?;
                    let mut times = BitSet::with_capacity(self.files.len());
                    for i in 0..self.files.len() {
                        if self.files[i].$has_time {
                            times.insert(i);
                        }
                    }
                    write_bit_set(&mut out, &times)?;
                } else {
                    out.write_u8(1)?;
                }
                out.write_u8(0)?;
                for file in self.files.iter() {
                    if file.$has_time {
                        out.$write_fn::<LittleEndian>((file.$time).into())?;
                    }
                }
                out.flush()?;
                write_u64(header, temp.len() as u64)?;
                header.write_all(&temp)?;
            }
            Ok(())
        }
    };
}

type Result<T> = std::result::Result<T, crate::Error>;

/// Writes a 7z file
pub struct SevenZWriter<W: Write> {
    output: W,
    files: Vec<SevenZArchiveEntry>,
    content_methods: Arc<Vec<SevenZMethodConfiguration>>,
    pack_info: PackInfo,
    unpack_info: UnpackInfo,
}

#[cfg(not(target_arch = "wasm32"))]
impl SevenZWriter<File> {
    /// Creates a file to write a 7z archive to
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::File::create(path.as_ref())
            .map_err(|e| crate::Error::file_open(e, path.as_ref().to_string_lossy().to_string()))?;
        Self::new(file)
    }
}
impl<W: Write + Seek> SevenZWriter<W> {
    /// Prepares writer to write a 7z archive to
    pub fn new(mut writer: W) -> Result<Self> {
        writer
            .seek(std::io::SeekFrom::Start(
                crate::archive::SIGNATURE_HEADER_SIZE,
            ))
            .map_err(Error::io)?;

        Ok(Self {
            output: writer,
            files: Default::default(),
            content_methods: Arc::new(vec![SevenZMethodConfiguration::new(SevenZMethod::LZMA2)]),
            pack_info: Default::default(),
            unpack_info: Default::default(),
        })
    }

    /// Sets the default compression methods to use for entry contents.
    /// The default is LZMA2.
    /// And currently only support LZMA2
    ///
    pub fn set_content_methods(
        &mut self,
        content_methods: Vec<SevenZMethodConfiguration>,
    ) -> &mut Self {
        if content_methods.is_empty() {
            return self;
        }
        self.content_methods = Arc::new(content_methods);
        self
    }

    /// Create an archive entry using the file in `path` and entry_name provided.
    /// #deprecated use SevenZArchiveEntry::from_path instead
    #[deprecated]
    pub fn create_archive_entry(path: impl AsRef<Path>, entry_name: String) -> SevenZArchiveEntry {
        let path = path.as_ref();

        let mut entry = SevenZArchiveEntry {
            name: entry_name,
            has_stream: path.is_file(),
            is_directory: path.is_dir(),
            ..Default::default()
        };

        if let Ok(meta) = path.metadata() {
            if let Ok(modified) = meta.modified() {
                entry.last_modified_date = modified
                    .try_into()
                    .expect("last modified date should be in the range of file time");
                entry.has_last_modified_date = entry.last_modified_date.to_raw() > 0;
            }
        }
        entry
    }

    /// Adds an archive `entry` with data from `reader`
    pub fn push_archive_entry<R: Read>(
        &mut self,
        mut entry: SevenZArchiveEntry,
        reader: Option<R>,
    ) -> Result<&SevenZArchiveEntry> {
        if !entry.is_directory {
            if let Some(mut r) = reader {
                let mut compressed_len = 0;
                let mut compressed = CompressWrapWriter::new(&mut self.output, &mut compressed_len);
                let content_methods = if entry.content_methods.is_empty() {
                    &self.content_methods
                } else {
                    &entry.content_methods
                };
                let mut more_sizes: Vec<Rc<Cell<usize>>> =
                    Vec::with_capacity(content_methods.len() - 1);

                let (crc, size) = {
                    let mut w =
                        Self::create_writer(content_methods, &mut compressed, &mut more_sizes)?;
                    let mut write_len = 0;
                    let mut w = CompressWrapWriter::new(&mut w, &mut write_len);
                    let mut buf = [0u8; 4096];
                    loop {
                        match r.read(&mut buf) {
                            Ok(n) => {
                                if n == 0 {
                                    break;
                                }
                                w.write_all(&buf[..n]).map_err(|e| {
                                    Error::io_msg(e, format!("Encode entry:{}", entry.name()))
                                })?;
                            }
                            Err(e) => {
                                return Err(Error::io_msg(
                                    e,
                                    format!("Encode entry:{}", entry.name()),
                                ));
                            }
                        }
                    }
                    w.flush()
                        .map_err(|e| Error::io_msg(e, format!("Encode entry:{}", entry.name())))?;
                    w.write(&[])
                        .map_err(|e| Error::io_msg(e, format!("Encode entry:{}", entry.name())))?;

                    (w.crc_value(), write_len)
                };
                let compressed_crc = compressed.crc_value();
                entry.has_stream = true;
                entry.size = size as u64;
                entry.crc = crc as u64;
                entry.has_crc = true;
                entry.compressed_crc = compressed_crc as u64;
                entry.compressed_size = compressed_len as u64;
                self.pack_info
                    .add_stream(compressed_len as u64, compressed_crc);

                let mut sizes = Vec::with_capacity(more_sizes.len() + 1);
                sizes.extend(more_sizes.iter().map(|s| s.get() as u64));
                sizes.push(size as u64);

                self.unpack_info.add(content_methods.clone(), sizes, crc);

                self.files.push(entry);
                return Ok(self.files.last().unwrap());
            }
        }
        entry.has_stream = false;
        entry.size = 0;
        entry.compressed_size = 0;
        entry.has_crc = false;
        self.files.push(entry);
        Ok(self.files.last().unwrap())
    }

    /// [Solid compression](https://en.wikipedia.org/wiki/Solid_compression)
    /// pack [entries] into one pack
    /// # Panics
    /// Panics if `entries`'s length not equals to `reader.reader_len()`
    pub fn push_archive_entries<R: Read>(
        &mut self,
        mut entries: Vec<SevenZArchiveEntry>,
        reader: SeqReader<SourceReader<R>>,
    ) -> Result<&mut Self> {
        let mut r = reader;
        assert_eq!(r.reader_len(), entries.len());
        let mut compressed_len = 0;
        let mut compressed = CompressWrapWriter::new(&mut self.output, &mut compressed_len);
        let content_methods = &self.content_methods;
        let mut more_sizes: Vec<Rc<Cell<usize>>> = Vec::with_capacity(content_methods.len() - 1);

        let (crc, size) = {
            let mut w = Self::create_writer(content_methods, &mut compressed, &mut more_sizes)?;
            let mut write_len = 0;
            let mut w = CompressWrapWriter::new(&mut w, &mut write_len);
            let mut buf = [0u8; 4096];
            fn entries_names(entries: &[SevenZArchiveEntry]) -> String {
                let mut names = String::with_capacity(512);
                for ele in entries.iter() {
                    names.push_str(&ele.name);
                    names.push(';');
                    if names.len() > 512 {
                        break;
                    }
                }
                names
            }
            loop {
                match r.read(&mut buf) {
                    Ok(n) => {
                        if n == 0 {
                            break;
                        }
                        w.write_all(&buf[..n]).map_err(|e| {
                            Error::io_msg(e, format!("Encode entries:{}", entries_names(&entries)))
                        })?;
                    }
                    Err(e) => {
                        return Err(Error::io_msg(
                            e,
                            format!("Encode entries:{}", entries_names(&entries)),
                        ));
                    }
                }
            }
            w.flush().map_err(|e| {
                let mut names = String::with_capacity(512);
                for ele in entries.iter() {
                    names.push_str(&ele.name);
                    names.push(';');
                    if names.len() > 512 {
                        break;
                    }
                }
                Error::io_msg(e, format!("Encode entry:{}", names))
            })?;
            w.write(&[]).map_err(|e| {
                Error::io_msg(e, format!("Encode entry:{}", entries_names(&entries)))
            })?;

            (w.crc_value(), write_len)
        };
        let compressed_crc = compressed.crc_value();
        let mut sub_stream_crcs = Vec::with_capacity(entries.len());
        let mut sub_stream_sizes = Vec::with_capacity(entries.len());
        for i in 0..entries.len() {
            let entry = &mut entries[i];
            let ri = &r[i];
            entry.crc = ri.crc_value() as u64;
            entry.size = ri.read_count() as u64;
            sub_stream_crcs.push(entry.crc as u32);
            sub_stream_sizes.push(entry.size as u64);
            entry.has_crc = true;
        }

        self.pack_info
            .add_stream(compressed_len as u64, compressed_crc);

        let mut sizes = Vec::with_capacity(more_sizes.len() + 1);
        sizes.extend(more_sizes.iter().map(|s| s.get() as u64));
        sizes.push(size as u64);

        self.unpack_info.add_multiple(
            content_methods.clone(),
            sizes,
            crc,
            entries.len() as u64,
            sub_stream_sizes,
            sub_stream_crcs,
        );

        self.files.extend(entries);
        return Ok(self);
    }

    fn create_writer<'a, O: Write + 'a>(
        methods: &[SevenZMethodConfiguration],
        out: O,
        more_sized: &mut Vec<Rc<Cell<usize>>>,
    ) -> Result<Box<dyn Write + 'a>> {
        let mut encoder: Box<dyn Write> = Box::new(out);
        let mut first = true;
        for mc in methods.iter() {
            if !first {
                let counting = CountingWriter::new(encoder);
                more_sized.push(counting.counting());
                encoder = Box::new(encoders::add_encoder(counting, mc)?);
            } else {
                let counting = CountingWriter::new(encoder);
                encoder = Box::new(encoders::add_encoder(counting, mc)?);
            }
            first = false;
        }
        Ok(encoder)
    }

    /// Finishes the compression.
    pub fn finish(mut self) -> std::io::Result<W> {
        let mut header: Vec<u8> = Vec::with_capacity(64 * 1024);
        self.write_encoded_header(&mut header)?;
        let header_pos = self.output.stream_position()?;
        self.output.write_all(&header)?;
        let crc32 = CRC32.checksum(&header);
        let mut hh = [0u8; SIGNATURE_HEADER_SIZE as usize];
        {
            let mut hhw = hh.as_mut_slice();
            //sig
            hhw.write_all(&SEVEN_Z_SIGNATURE)?;
            //version
            hhw.write_u8(0)?;
            hhw.write_u8(2)?;
            //placeholder for crc: index = 8
            hhw.write_u32::<LittleEndian>(0)?;

            // start header
            hhw.write_u64::<LittleEndian>(header_pos - SIGNATURE_HEADER_SIZE)?;
            hhw.write_u64::<LittleEndian>(0xffffffff & header.len() as u64)?;
            hhw.write_u32::<LittleEndian>(crc32)?;
        }
        let crc32 = CRC32.checksum(&hh[12..]);
        hh[8..12].copy_from_slice(&crc32.to_le_bytes());

        self.output.seek(std::io::SeekFrom::Start(0))?;
        self.output.write(&hh)?;
        Ok(self.output)
    }

    fn write_header<H: Write>(&mut self, header: &mut H) -> std::io::Result<()> {
        header.write_u8(K_HEADER)?;
        header.write_u8(K_MAIN_STREAMS_INFO)?;
        self.write_streams_info(header)?;
        self.write_files_info(header)?;
        header.write_u8(K_END)?;
        Ok(())
    }

    fn write_encoded_header<H: Write>(&mut self, header: &mut H) -> std::io::Result<()> {
        let mut raw_header = Vec::with_capacity(64 * 1024);
        self.write_header(&mut raw_header)?;
        let mut pack_info = PackInfo::default();

        let position = self.output.stream_position()?;
        let pos = position - SIGNATURE_HEADER_SIZE;
        pack_info.pos = pos;

        let mut more_sizes = vec![];
        let size = raw_header.len() as u64;
        let crc = CRC32.checksum(&raw_header);
        let methods = vec![SevenZMethodConfiguration::new(SevenZMethod::LZMA)
            .with_options(LZMA2Options::with_preset(6).into())];

        let methods = Arc::new(methods);

        let mut encoded_data = Vec::with_capacity(size as usize / 2);

        let mut compress_size = 0;
        let mut compressed = CompressWrapWriter::new(&mut encoded_data, &mut compress_size);

        {
            let mut encoder = Self::create_writer(&methods, &mut compressed, &mut more_sizes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            encoder.write_all(&raw_header)?;
            encoder.write(&[])?;
        }
        let compress_crc = compressed.crc_value();
        let compress_size = *compressed.bytes_written;
        if compress_size as u64 + 20 >= size {
            // compression made it worse. Write raw data
            header.write_all(&raw_header)?;
            return Ok(());
        }
        self.output.write_all(&encoded_data[..compress_size])?;

        pack_info.add_stream(compress_size as u64, compress_crc);

        let mut unpack_info = UnpackInfo::default();
        let mut sizes = vec![size];
        sizes.extend(more_sizes.iter().map(|s| s.get() as u64));
        unpack_info.add(methods, sizes, crc);

        header.write_u8(K_ENCODED_HEADER)?;

        pack_info.write_to(header)?;
        unpack_info.write_to(header)?;
        unpack_info.write_substreams(header)?;

        header.write_u8(K_END)?;

        Ok(())
    }

    fn write_streams_info<H: Write>(&mut self, header: &mut H) -> std::io::Result<()> {
        if self.pack_info.len() > 0 {
            self.pack_info.write_to(header)?;
            self.unpack_info.write_to(header)?;
        }
        self.unpack_info.write_substreams(header)?;

        header.write_u8(K_END)?;
        Ok(())
    }

    fn write_files_info<H: Write>(&self, header: &mut H) -> std::io::Result<()> {
        header.write_u8(K_FILES_INFO)?;
        write_u64(header, self.files.len() as u64)?;
        self.write_file_empty_streams(header)?;
        self.write_file_empty_files(header)?;
        self.write_file_anti_items(header)?;
        self.write_file_names(header)?;
        self.write_file_ctimes(header)?;
        self.write_file_atimes(header)?;
        self.write_file_mtimes(header)?;
        self.write_file_windows_attrs(header)?;
        header.write_u8(K_END)?;
        Ok(())
    }

    fn write_file_empty_streams<H: Write>(&self, header: &mut H) -> std::io::Result<()> {
        let mut has_empty = false;
        for entry in self.files.iter() {
            if !entry.has_stream {
                has_empty = true;
                break;
            }
        }
        if has_empty {
            header.write_u8(K_EMPTY_STREAM)?;
            let mut bitset = BitSet::with_capacity(self.files.len());
            let mut i = 0;
            for entry in self.files.iter() {
                if !entry.has_stream {
                    bitset.insert(i);
                }
                i += 1;
            }
            let mut temp: Vec<u8> = Vec::with_capacity(bitset.len() / 8 + 1);
            write_bit_set(&mut temp, &bitset)?;
            write_u64(header, temp.len() as u64)?;
            header.write(temp.as_slice())?;
        }
        Ok(())
    }
    fn write_file_empty_files<H: Write>(&self, header: &mut H) -> std::io::Result<()> {
        let mut has_empty = false;
        let mut empty_stream_counter = 0;
        let mut bitset = BitSet::new();
        for entry in self.files.iter() {
            if !entry.has_stream {
                let is_dir = entry.is_directory();
                has_empty = has_empty | !is_dir;
                if !is_dir {
                    bitset.insert(empty_stream_counter);
                }
                empty_stream_counter += 1;
            }
        }
        if has_empty {
            header.write_u8(K_EMPTY_FILE)?;

            let mut temp: Vec<u8> = Vec::with_capacity(bitset.len() / 8 + 1);
            write_bit_set(&mut temp, &bitset)?;
            write_u64(header, temp.len() as u64)?;
            header.write(temp.as_slice())?;
        }
        Ok(())
    }

    fn write_file_anti_items<H: Write>(&self, header: &mut H) -> std::io::Result<()> {
        let mut has_anti = false;
        let mut counter = 0;
        let mut bitset = BitSet::new();
        for entry in self.files.iter() {
            if !entry.has_stream {
                let is_anti = entry.is_anti_item();
                has_anti = has_anti | !is_anti;
                if !is_anti {
                    bitset.insert(counter);
                }
                counter += 1;
            }
        }
        if has_anti {
            header.write_u8(K_ANTI)?;

            let mut temp: Vec<u8> = Vec::with_capacity(bitset.len() / 8 + 1);
            write_bit_set(&mut temp, &bitset)?;
            write_u64(header, temp.len() as u64)?;
            header.write(temp.as_slice())?;
        }
        Ok(())
    }
    fn write_file_names<H: Write>(&self, header: &mut H) -> std::io::Result<()> {
        header.write_u8(K_NAME)?;
        let mut temp: Vec<u8> = Vec::with_capacity(128);
        let out = &mut temp;
        out.write_u8(0)?;
        for file in self.files.iter() {
            for c in file.name().encode_utf16() {
                let buf = c.to_le_bytes();
                out.write_all(&buf)?;
            }
            out.write_all(&[0u8; 2])?;
        }
        write_u64(header, temp.len() as u64)?;
        header.write_all(temp.as_slice())?;
        Ok(())
    }

    write_times!(
        write_file_ctimes,
        K_C_TIME,
        has_creation_date,
        creation_date
    );
    write_times!(write_file_atimes, K_A_TIME, has_access_date, access_date);
    write_times!(
        write_file_mtimes,
        K_M_TIME,
        has_last_modified_date,
        last_modified_date
    );
    write_times!(
        write_file_windows_attrs,
        K_WIN_ATTRIBUTES,
        has_windows_attributes,
        windows_attributes,
        write_u32
    );
}

pub(crate) fn write_u64<W: Write>(header: &mut W, mut value: u64) -> std::io::Result<()> {
    let mut first = 0;
    let mut mask = 0x80;
    let mut i = 0;
    while i < 8 {
        if value < (1u64 << (7 * (i + 1))) {
            first |= value >> (8 * i);
            break;
        }
        first |= mask;
        mask = mask >> 1;
        i += 1;
    }
    header.write_u8((first & 0xff) as u8)?;
    while i > 0 {
        header.write_u8((value & 0xff) as u8)?;
        value = value >> 8;
        i -= 1;
    }
    Ok(())
}

fn write_bit_set<W: Write>(mut write: W, bs: &BitSet) -> std::io::Result<()> {
    let mut cache = 0;
    let mut shift = 7;
    for i in 0..bs.get_ref().len() {
        let set = if bs.contains(i) { 1 } else { 0 };
        cache |= set << shift;
        shift -= 1;
        if shift < 0 {
            write.write_u8(cache)?;
            shift = 7;
            cache = 0;
        }
    }
    if shift != 7 {
        write.write_u8(cache)?;
    }
    Ok(())
}

struct CompressWrapWriter<'a, W> {
    writer: W,
    crc: crc::Digest<'static, u32>,
    cache: Vec<u8>,
    bytes_written: &'a mut usize,
}
impl<'a, W: Write> CompressWrapWriter<'a, W> {
    pub fn new(writer: W, bytes_written: &'a mut usize) -> Self {
        Self {
            writer,
            crc: crate::reader::CRC32.digest(),
            cache: Vec::with_capacity(8192),
            bytes_written,
        }
    }

    pub fn crc_value(&mut self) -> u32 {
        let crc = std::mem::replace(&mut self.crc, crate::reader::CRC32.digest());
        crc.finalize()
    }
}

impl<'a, W: Write> Write for CompressWrapWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.cache.resize(buf.len(), Default::default());
        let len = self.writer.write(buf)?;
        self.crc.update(&buf[..len]);
        *self.bytes_written = *self.bytes_written + len;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
