use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
};

use bit_set::BitSet;
use crc::Crc;

use crate::{archive::*, decoders::add_decoder, error::Error, folder::*, password::Password};
pub(crate) const CRC32: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
const MAX_MEM_LIMIT_KB: usize = usize::MAX / 1024;

pub(crate) trait SeedRead: Read + Seek {}

pub struct BoundedReader<R: Read> {
    inner: R,
    remain: usize,
}

impl<R: Read> BoundedReader<R> {
    pub fn new(inner: R, max_size: usize) -> Self {
        Self {
            inner,
            remain: max_size,
        }
    }
}

impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remain == 0 {
            return Ok(0);
        }
        let remain = self.remain;
        let buf2 = if buf.len() < remain {
            buf
        } else {
            &mut buf[..remain]
        };
        match self.inner.read(buf2) {
            Ok(size) => {
                if self.remain < size {
                    self.remain = 0;
                } else {
                    self.remain -= size;
                }
                Ok(size)
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SeekableBoundedReader<R: Read + Seek> {
    inner: R,
    cur: u64,
    bounds: (u64, u64),
}

impl<R: Read + Seek> SeedRead for SeekableBoundedReader<R> {}

impl<R: Read + Seek> Seek for SeekableBoundedReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(pos) => self.bounds.0 as i64 + pos as i64,
            SeekFrom::End(pos) => self.bounds.1 as i64 + pos,
            SeekFrom::Current(pos) => self.cur as i64 + pos,
        };
        if new_pos < 0 {
            return Err(std::io::Error::new(ErrorKind::Other, "SeekBeforeStart"));
        }
        self.cur = new_pos as u64;
        self.inner.seek(SeekFrom::Start(self.cur))
    }
}

impl<R: Read + Seek> Read for SeekableBoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cur >= self.bounds.1 {
            return Ok(0);
        }
        if self.stream_position()? != self.cur {
            println!("seeking to {}", self.cur);
            self.inner.seek(SeekFrom::Start(self.cur))?;
        }
        let buf2 = if buf.len() < (self.bounds.1 - self.cur) as usize {
            buf
        } else {
            &mut buf[..(self.bounds.1 - self.cur) as usize]
        };
        let size = self.inner.read(buf2)?;
        self.cur += size as u64;
        Ok(size)
    }
}

impl<R: Read + Seek> SeekableBoundedReader<R> {
    pub fn new(inner: R, bounds: (u64, u64)) -> Self {
        Self {
            inner,
            cur: bounds.0,
            bounds,
        }
    }
}

struct Crc32VerifyingReader<R> {
    inner: R,
    crc_digest: crc::Digest<'static, u32>,
    expected_value: u64,
    remaining: i64,
}

impl<R: Read> Crc32VerifyingReader<R> {
    fn new(inner: R, remaining: usize, expected_value: u64) -> Self {
        Self {
            inner,
            crc_digest: CRC32.digest(),
            expected_value,
            remaining: remaining as i64,
        }
    }
}

impl<R: Read> Read for Crc32VerifyingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining <= 0 {
            return Ok(0);
        }
        let size = self.inner.read(buf)?;
        if size > 0 {
            self.remaining -= size as i64;
            self.crc_digest.update(&buf[..size]);
        }
        if self.remaining <= 0 {
            let d = std::mem::replace(&mut self.crc_digest, CRC32.digest()).finalize();
            if d as u64 != self.expected_value {
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    Error::ChecksumVerificationFailed,
                ));
            }
        }
        Ok(size)
    }
}

impl Archive {
    pub fn read<R: Read + Seek>(
        reader: &mut R,
        reader_len: u64,
        password: &[u8],
    ) -> Result<Archive, Error> {
        let mut signature = [0; 6];
        reader.read_exact(&mut signature).map_err(Error::io)?;
        if signature != SEVEN_Z_SIGNATURE {
            return Err(Error::BadSignature(signature));
        }
        let mut versions = [0; 2];
        reader.read_exact(&mut versions).map_err(Error::io)?;
        let version_major = versions[0];
        let version_minor = versions[1];
        if version_major != 0 {
            return Err(Error::UnsupportedVersion {
                major: version_major,
                minor: version_minor,
            });
        }

        let start_header_crc = 0xffffffff & read_u32(reader)?;

        let header_valid = if start_header_crc == 0 {
            let current_position = reader.stream_position().map_err(Error::io)?;
            let mut buf = [0; 20];
            reader.read_exact(&mut buf).map_err(Error::io)?;
            reader
                .seek(std::io::SeekFrom::Start(current_position))
                .map_err(Error::io)?;
            buf.iter().any(|a| *a != 0)
        } else {
            true
        };
        if header_valid {
            let start_header = Self::read_start_header(reader, start_header_crc)?;
            Self::init_archive(reader, start_header, password, true)
        } else {
            Self::try_to_locale_end_header(reader, reader_len, password)
        }
    }

    fn read_start_header<R: Read>(
        reader: &mut R,
        start_header_crc: u32,
    ) -> Result<StartHeader, Error> {
        let mut buf = [0; 20];
        reader.read_exact(&mut buf).map_err(Error::io)?;
        let value = crc32_cksum(&buf);
        if value != start_header_crc {
            return Err(Error::ChecksumVerificationFailed);
        }
        let mut buf_read = buf.as_slice();
        let offset = read_u64le(&mut buf_read)?;

        let size = read_u64le(&mut buf_read)?;
        let crc = read_u32(&mut buf_read)?;
        Ok(StartHeader {
            next_header_offset: offset,
            next_header_size: size,
            next_header_crc: crc as u64,
        })
    }

    fn read_header<R: Read + Seek>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let mut nid = read_u8(header)?;
        if nid == K_ARCHIVE_PROPERTIES {
            Self::read_archive_properties(header)?;
            nid = read_u8(header)?;
        }

        if nid == K_ADDITIONAL_STREAMS_INFO {
            return Err(Error::other("Additional streams unsupported"));
        }
        if nid == K_MAIN_STREAMS_INFO {
            Self::read_streams_info(header, archive)?;
            nid = read_u8(header)?;
        }
        if nid == K_FILES_INFO {
            Self::read_files_info(header, archive)?;
            nid = read_u8(header)?;
        }
        if nid != K_END {
            return Err(Error::BadTerminatedheader(nid));
        }

        Ok(())
    }

    fn read_archive_properties<R: Read + Seek>(header: &mut R) -> Result<(), Error> {
        let mut nid = read_u8(header)?;
        while nid != K_END {
            let property_size = read_usize(header, "propertySize")?;
            header
                .seek(SeekFrom::Current(property_size as i64))
                .map_err(Error::io)?;
            nid = read_u8(header)?;
        }
        Ok(())
    }

    fn try_to_locale_end_header<R: Read + Seek>(
        reader: &mut R,
        reader_len: u64,
        password: &[u8],
    ) -> Result<Self, Error> {
        let search_limit = 1024 * 1024 * 1;
        let prev_data_size = reader.stream_position().map_err(Error::io)? + 20;
        let size = reader_len;
        let min_pos = if reader.stream_position().map_err(Error::io)? + search_limit > size {
            reader.stream_position().map_err(Error::io)?
        } else {
            size - search_limit
        };
        let mut pos = reader_len - 1;
        while pos > min_pos {
            pos -= 1;

            reader
                .seek(std::io::SeekFrom::Start(pos))
                .map_err(Error::io)?;
            let nid = read_u8(reader)?;
            if nid == K_ENCODED_HEADER || nid == K_HEADER {
                let start_header = StartHeader {
                    next_header_offset: pos - prev_data_size,
                    next_header_size: reader_len - pos,
                    next_header_crc: 0,
                };
                let result = Self::init_archive(reader, start_header, password, false)?;

                if result.files.len() > 0 {
                    return Ok(result);
                }
            }
        }
        Err(Error::other(
            "Start header corrupt and unable to guess end header",
        ))
    }

    fn init_archive<R: Read + Seek>(
        reader: &mut R,
        start_header: StartHeader,
        password: &[u8],
        verify_crc: bool,
    ) -> Result<Self, Error> {
        if start_header.next_header_size > usize::MAX as u64 {
            return Err(Error::other(format!(
                "Cannot handle next_header_size {}",
                start_header.next_header_size
            )));
        }

        let next_header_size_int = start_header.next_header_size as usize;

        reader
            .seek(SeekFrom::Start(
                SIGNATURE_HEADER_SIZE + start_header.next_header_offset,
            ))
            .map_err(Error::io)?;

        let mut buf = vec![0; next_header_size_int];
        reader.read_exact(&mut buf).map_err(Error::io)?;
        if verify_crc {
            if crc32_cksum(&buf) as u64 != start_header.next_header_crc {
                return Err(Error::NextHeaderCrcMismatch);
            }
        }

        let mut archive = Archive::default();
        let mut buf_reader = buf.as_slice();
        let mut nid = read_u8(&mut buf_reader)?;
        let mut header = if nid == K_ENCODED_HEADER {
            let (mut out_reader, buf_size) =
                Self::read_encoded_header(&mut buf_reader, reader, &mut archive, password)?;
            buf.clear();
            buf.resize(buf_size, 0);
            out_reader.read_exact(&mut buf).map_err(Error::io)?;
            archive = Archive::default();
            buf_reader = buf.as_slice();
            nid = read_u8(&mut buf_reader)?;
            buf_reader
        } else {
            buf_reader
        };
        let mut header = std::io::Cursor::new(&mut header);
        if nid == K_HEADER {
            Self::read_header(&mut header, &mut archive)?;
        } else {
            return Err(Error::other("Broken or unsupported archive: no Header"));
        }
        Ok(archive)
    }

    fn read_encoded_header<'r, R: Read, RI: 'r + Read + Seek>(
        header: &mut R,
        reader: &'r mut RI,
        archive: &mut Archive,
        password: &[u8],
    ) -> Result<(Box<dyn Read + 'r>, usize), Error> {
        Self::read_streams_info(header, archive)?;
        let folder = archive
            .folders
            .first()
            .ok_or(Error::other("no folders, can't read encoded header"))?;
        let first_pack_stream_index = 0;
        let folder_offset = SIGNATURE_HEADER_SIZE + archive.pack_pos + 0;
        if archive.pack_sizes.is_empty() {
            return Err(Error::other("no packed streams, can't read encoded header"));
        }

        reader
            .seek(SeekFrom::Start(folder_offset))
            .map_err(Error::io)?;
        let coder_len = folder.coders.len();
        let unpack_size = folder.get_unpack_size() as usize;
        let pack_size = archive.pack_sizes[first_pack_stream_index] as usize;
        let input_reader =
            SeekableBoundedReader::new(reader, (folder_offset, folder_offset + pack_size as u64));
        let mut decoder: Box<dyn Read> = Box::new(input_reader);
        let mut decoder = if coder_len > 0 {
            for (index, coder) in folder.ordered_coder_iter() {
                if coder.num_in_streams != 1 || coder.num_out_streams != 1 {
                    return Err(Error::other(
                        "Multi input/output stream coders are not yet supported",
                    ));
                }
                let next = crate::decoders::add_decoder(
                    decoder,
                    folder.get_unpack_size_at_index(index) as usize,
                    coder,
                    password,
                    MAX_MEM_LIMIT_KB,
                )?;
                decoder = Box::new(next);
            }
            decoder
        } else {
            decoder
        };
        if folder.has_crc {
            decoder = Box::new(Crc32VerifyingReader::new(decoder, unpack_size, folder.crc));
        }

        Ok((decoder, unpack_size))
    }

    fn read_streams_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let mut nid = read_u8(header)?;
        if nid == K_PACK_INFO {
            Self::read_pack_info(header, archive)?;
            nid = read_u8(header)?;
        }

        if nid == K_UNPACK_INFO {
            Self::read_unpack_info(header, archive)?;
            nid = read_u8(header)?;
        } else {
            archive.folders.clear();
        }
        if nid == K_SUB_STREAMS_INFO {
            Self::read_sub_streams_info(header, archive)?;
            nid = read_u8(header)?;
        }
        if nid != K_END {
            return Err(Error::BadTerminatedStreamsInfo(nid));
        }

        Ok(())
    }

    fn read_files_info<R: Read + Seek>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let num_files = read_usize(header, "num files")?;
        let mut files: Vec<SevenZArchiveEntry> = vec![Default::default(); num_files];

        let mut is_empty_stream: Option<BitSet> = None;
        let mut is_empty_file: Option<BitSet> = None;
        let mut is_anti: Option<BitSet> = None;
        loop {
            let prop_type = read_u8(header)?;
            if prop_type == 0 {
                break;
            }
            let size = read_u64(header)?;
            match prop_type {
                K_EMPTY_STREAM => {
                    is_empty_stream = Some(read_bits(header, num_files)?);
                }
                K_EMPTY_FILE => {
                    let n = if let Some(s) = &is_empty_stream {
                        s.len()
                    } else {
                        return Err(Error::other(
                            "Header format error: kEmptyStream must appear before kEmptyFile",
                        ));
                    };
                    is_empty_file = Some(read_bits(header, n)?);
                }
                K_ANTI => {
                    let n = if let Some(s) = is_empty_stream.as_ref() {
                        s.len()
                    } else {
                        return Err(Error::other(
                            "Header format error: kEmptyStream must appear before kEmptyFile",
                        ));
                    };
                    is_anti = Some(read_bits(header, n)?);
                }
                K_NAME => {
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other("Not implemented:external != 0"));
                    }
                    if (size - 1) & 1 != 0 {
                        return Err(Error::other("file names length invalid"));
                    }

                    let size = assert_usize(size, "file names length")?;
                    // let mut names = vec![0u8; size - 1];
                    // header.read_exact(&mut names).map_err(Error::io)?;
                    let mut names_reader = NamesReader::new(header, size - 1);

                    let mut next_file = 0;
                    while let Some(s) = names_reader.next() {
                        files[next_file].name = s?;
                        next_file += 1;
                    }

                    if next_file != files.len() {
                        return Err(Error::other("Error parsing file names"));
                    }
                }
                K_C_TIME => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kCTime Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_creation_date = times_defined.contains(i);
                        if files[i].has_creation_date {
                            files[i].creation_date = read_u64le(header)?.into();
                        }
                    }
                }
                K_A_TIME => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kATime Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_access_date = times_defined.contains(i);
                        if files[i].has_access_date {
                            files[i].access_date = read_u64le(header)?.into();
                        }
                    }
                }
                K_M_TIME => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kMTime Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_last_modified_date = times_defined.contains(i);
                        if files[i].has_last_modified_date {
                            files[i].last_modified_date = read_u64le(header)?.into();
                        }
                    }
                }
                K_WIN_ATTRIBUTES => {
                    let times_defined = read_all_or_bits(header, num_files)?;
                    let external = read_u8(header)?;
                    if external != 0 {
                        return Err(Error::other(format!(
                            "kWinAttributes Unimplemented:external={}",
                            external
                        )));
                    }
                    for i in 0..num_files {
                        files[i].has_windows_attributes = times_defined.contains(i);
                        if files[i].has_windows_attributes {
                            files[i].windows_attributes = read_u32(header)?;
                        }
                    }
                }
                K_START_POS => return Err(Error::other("kStartPos is unsupported, please report")),
                K_DUMMY => {
                    header
                        .seek(SeekFrom::Current(size as i64))
                        .map_err(Error::io)?;
                }
                _ => {
                    header
                        .seek(SeekFrom::Current(size as i64))
                        .map_err(Error::io)?;
                }
            };
        }

        let mut non_empty_file_counter = 0;
        let mut empty_file_counter = 0;
        for i in 0..files.len() {
            let file = &mut files[i];
            file.has_stream = is_empty_stream
                .as_ref()
                .map(|s| !s.contains(i))
                .unwrap_or(true);
            if file.has_stream {
                let sub_stream_info = if let Some(s) = archive.sub_streams_info.as_ref() {
                    s
                } else {
                    return Err(Error::other(
                        "Archive contains file with streams but no subStreamsInfo",
                    ));
                };
                file.is_directory = false;
                file.is_anti_item = false;
                file.has_crc = sub_stream_info.has_crc.contains(non_empty_file_counter);
                file.crc = sub_stream_info.crcs[non_empty_file_counter];
                file.size = sub_stream_info.unpack_sizes[non_empty_file_counter];
                non_empty_file_counter += 1;
            } else {
                file.is_directory = if let Some(s) = &is_empty_file {
                    !s.contains(empty_file_counter)
                } else {
                    true
                };
                file.is_anti_item = is_anti
                    .as_ref()
                    .map(|s| s.contains(empty_file_counter))
                    .unwrap_or(false);
                file.has_crc = false;
                file.size = 0;
                empty_file_counter += 1;
            }
        }
        archive.files = files;

        Self::calculate_stream_map(archive)?;
        Ok(())
    }

    fn calculate_stream_map(archive: &mut Archive) -> Result<(), Error> {
        let mut stream_map = StreamMap::default();

        let mut next_folder_pack_stream_index = 0;
        let num_folders = archive.folders.len();
        stream_map.folder_first_pack_stream_index = vec![0; num_folders];
        for i in 0..num_folders {
            stream_map.folder_first_pack_stream_index[i] = next_folder_pack_stream_index;
            next_folder_pack_stream_index += archive.folders[i].packed_streams.len();
        }

        let mut next_pack_stream_offset = 0;
        let num_pack_sizes = archive.pack_sizes.len();
        stream_map.pack_stream_offsets = vec![0; num_pack_sizes];
        for i in 0..num_pack_sizes {
            stream_map.pack_stream_offsets[i] = next_pack_stream_offset;
            next_pack_stream_offset += archive.pack_sizes[i];
        }

        stream_map.folder_first_file_index = vec![0; num_folders];
        stream_map.file_folder_index = vec![None; archive.files.len()];
        let mut next_folder_index = 0;
        let mut next_folder_unpack_stream_index = 0;
        for i in 0..archive.files.len() {
            if !archive.files[i].has_stream && next_folder_unpack_stream_index == 0 {
                stream_map.file_folder_index[i] = None;
                continue;
            }
            if next_folder_unpack_stream_index == 0 {
                while next_folder_index < archive.folders.len() {
                    stream_map.folder_first_file_index[next_folder_index] = i;
                    if archive.folders[next_folder_index].num_unpack_sub_streams > 0 {
                        break;
                    }
                    next_folder_index += 1;
                }
                if next_folder_index >= archive.folders.len() {
                    return Err(Error::other("Too few folders in archive"));
                }
            }
            stream_map.file_folder_index[i] = Some(next_folder_index);
            if !archive.files[i].has_stream {
                continue;
            }
            next_folder_unpack_stream_index += 1;
            if next_folder_unpack_stream_index
                >= archive.folders[next_folder_index].num_unpack_sub_streams
            {
                next_folder_index += 1;
                next_folder_unpack_stream_index = 0;
            }
        }

        archive.stream_map = stream_map;
        Ok(())
    }

    fn read_pack_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        archive.pack_pos = read_u64(header)?;
        let num_pack_streams = read_usize(header, "num pack streams")?;
        let mut nid = read_u8(header)?;
        if nid == K_SIZE {
            archive.pack_sizes = vec![0u64; num_pack_streams];
            for i in 0..archive.pack_sizes.len() {
                archive.pack_sizes[i] = read_u64(header)?;
            }
            nid = read_u8(header)?;
        }

        if nid == K_CRC {
            archive.pack_crcs_defined = read_all_or_bits(header, num_pack_streams)?;
            archive.pack_crcs = vec![0; num_pack_streams];
            for i in 0..num_pack_streams {
                if archive.pack_crcs_defined.contains(i) {
                    archive.pack_crcs[i] = read_u32(header)? as u64;
                }
            }
            nid = read_u8(header)?;
        }

        if nid != K_END {
            return Err(Error::BadTerminatedPackInfo(nid));
        }

        Ok(())
    }
    fn read_unpack_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        let nid = read_u8(header)?;
        if nid != K_FOLDER {
            return Err(Error::other(format!("Expected kFolder, got {}", nid)));
        }
        let num_folders = read_usize(header, "num folders")?;

        archive.folders.reserve_exact(num_folders);
        let external = read_u8(header)?;
        if external != 0 {
            return Err(Error::ExternalUnsupported);
        }

        for _ in 0..num_folders {
            archive.folders.push(Self::read_folder(header)?);
        }

        let nid = read_u8(header)?;
        if nid != K_CODERS_UNPACK_SIZE {
            return Err(Error::other(format!(
                "Expected kCodersUnpackSize, got {}",
                nid
            )));
        }

        for folder in archive.folders.iter_mut() {
            let tos = folder.total_output_streams;
            folder.unpack_sizes.reserve_exact(tos);
            for _ in 0..tos {
                folder.unpack_sizes.push(read_u64(header)?);
            }
        }

        let mut nid = read_u8(header)?;
        if nid == K_CRC {
            let crcs_defined = read_all_or_bits(header, num_folders)?;
            for i in 0..num_folders {
                if crcs_defined.contains(i) {
                    archive.folders[i].has_crc = true;
                    archive.folders[i].crc = read_u32(header)? as u64;
                } else {
                    archive.folders[i].has_crc = false;
                }
            }
            nid = read_u8(header)?;
        }
        if nid != K_END {
            return Err(Error::BadTerminatedUnpackInfo);
        }

        Ok(())
    }

    fn read_sub_streams_info<R: Read>(header: &mut R, archive: &mut Archive) -> Result<(), Error> {
        for folder in archive.folders.iter_mut() {
            folder.num_unpack_sub_streams = 1;
        }
        let mut total_unpack_streams = archive.folders.len();

        let mut nid = read_u8(header)?;
        if nid == K_NUM_UNPACK_STREAM {
            total_unpack_streams = 0;
            for folder in archive.folders.iter_mut() {
                let num_streams = read_usize(header, "numStreams")?;
                folder.num_unpack_sub_streams = num_streams;
                total_unpack_streams += num_streams;
            }
            nid = read_u8(header)?;
        }

        let mut sub_streams_info = SubStreamsInfo::default();
        sub_streams_info
            .unpack_sizes
            .resize(total_unpack_streams, Default::default());
        sub_streams_info
            .has_crc
            .reserve_len_exact(total_unpack_streams);
        sub_streams_info.crcs = vec![0; total_unpack_streams];

        let mut next_unpack_stream = 0;
        for folder in archive.folders.iter() {
            if folder.num_unpack_sub_streams == 0 {
                continue;
            }
            let mut sum = 0;
            if nid == K_SIZE {
                for _i in 0..folder.num_unpack_sub_streams - 1 {
                    let size = read_u64(header)?;
                    sub_streams_info.unpack_sizes[next_unpack_stream] = size;
                    next_unpack_stream += 1;
                    sum += size;
                }
            }
            if sum > folder.get_unpack_size() {
                return Err(Error::other(
                    "sum of unpack sizes of folder exceeds total unpack size",
                ));
            }
            sub_streams_info.unpack_sizes[next_unpack_stream] = folder.get_unpack_size() - sum;
            next_unpack_stream += 1;
        }
        if nid == K_SIZE {
            nid = read_u8(header)?;
        }

        let mut num_digests = 0;
        for folder in archive.folders.iter() {
            if folder.num_unpack_sub_streams != 1 || !folder.has_crc {
                num_digests += folder.num_unpack_sub_streams;
            }
        }

        if nid == K_CRC {
            let has_missing_crc = read_all_or_bits(header, num_digests)?;
            let mut missing_crcs = vec![0; num_digests];
            for i in 0..num_digests {
                if has_missing_crc.contains(i) {
                    missing_crcs[i] = read_u32(header)? as u64;
                }
            }
            let mut next_crc = 0;
            let mut next_missing_crc = 0;
            for folder in archive.folders.iter() {
                if folder.num_unpack_sub_streams == 1 && folder.has_crc {
                    sub_streams_info.has_crc.insert(next_crc);
                    sub_streams_info.crcs[next_crc] = folder.crc;
                    next_crc += 1;
                } else {
                    for _i in 0..folder.num_unpack_sub_streams {
                        if has_missing_crc.contains(next_missing_crc) {
                            sub_streams_info.has_crc.insert(next_crc);
                        } else {
                            sub_streams_info.has_crc.remove(next_crc);
                        }
                        sub_streams_info.crcs[next_crc] = missing_crcs[next_missing_crc];
                        next_crc += 1;
                        next_missing_crc += 1;
                    }
                }
            }

            nid = read_u8(header)?;
        }

        if nid != K_END {
            return Err(Error::BadTerminatedSubStreamsInfo);
        }

        archive.sub_streams_info = Some(sub_streams_info);
        Ok(())
    }

    fn read_folder<R: Read>(header: &mut R) -> Result<Folder, Error> {
        let mut folder = Folder::default();

        let num_coders = read_usize(header, "num coders")?;
        let mut coders = Vec::with_capacity(num_coders);
        let mut total_in_streams = 0;
        let mut total_out_streams = 0;
        for _i in 0..num_coders {
            let mut coder = Coder::default();
            let bits = read_u8(header)?;
            let id_size = bits & 0xf;
            let is_simple = (bits & 0x10) == 0;
            let has_attributes = (bits & 0x20) != 0;
            let more_alternative_methods = (bits & 0x80) != 0;

            coder.id_size = id_size as usize;

            header
                .read(&mut coder.decompression_method_id_mut())
                .map_err(Error::io)?;
            if is_simple {
                coder.num_in_streams = 1;
                coder.num_out_streams = 1;
            } else {
                coder.num_in_streams = read_u64(header)?;
                coder.num_out_streams = read_u64(header)?;
            }
            total_in_streams += coder.num_in_streams;
            total_out_streams += coder.num_out_streams;
            if has_attributes {
                let properties_size = read_usize(header, "properties size")?;
                let mut props = vec![0u8; properties_size];
                header.read(&mut props).map_err(Error::io)?;
                coder.properties = props;
            }
            coders.push(coder);
            // would need to keep looping as above:
            while more_alternative_methods {
                return Err(Error::other("Alternative methods are unsupported, please report. The reference implementation doesn't support them either."));
            }
        }
        folder.coders = coders;
        let total_in_streams = assert_usize(total_in_streams, "totalInStreams")?;
        let total_out_streams = assert_usize(total_out_streams, "totalOutStreams")?;
        folder.total_input_streams = total_in_streams;
        folder.total_output_streams = total_out_streams;

        if total_out_streams == 0 {
            return Err(Error::other("Total output streams can't be 0"));
        }
        let num_bind_pairs = total_out_streams - 1;
        let mut bind_pairs = Vec::with_capacity(num_bind_pairs);
        for _ in 0..num_bind_pairs {
            let bp = BindPair {
                in_index: read_u64(header)?,
                out_index: read_u64(header)?,
            };
            bind_pairs.push(bp);
        }
        folder.bind_pairs = bind_pairs;

        if total_in_streams < num_bind_pairs {
            return Err(Error::other(
                "Total input streams can't be less than the number of bind pairs",
            ));
        }
        let num_packed_streams = total_in_streams - num_bind_pairs;
        let mut packed_streams = vec![0; num_packed_streams];
        if num_packed_streams == 1 {
            let mut index = u64::MAX;
            for i in 0..total_in_streams {
                if folder.find_bind_pair_for_in_stream(i).is_none() {
                    index = i as u64;
                    break;
                }
            }
            if index == u64::MAX {
                return Err(Error::other("Couldn't find stream's bind pair index"));
            }
            packed_streams[0] = index;
        } else {
            for i in 0..num_packed_streams {
                packed_streams[i] = read_u64(header)?;
            }
        }
        folder.packed_streams = packed_streams;

        return Ok(folder);
    }
}

#[inline]
fn crc32_cksum(data: &[u8]) -> u32 {
    CRC32.checksum(data)
}

#[inline]
fn read_usize<R: Read>(reader: &mut R, field: &str) -> Result<usize, Error> {
    let size = read_u64(reader)?;
    assert_usize(size, field)
}

#[inline]
fn assert_usize(size: u64, field: &str) -> Result<usize, Error> {
    if size > usize::MAX as u64 {
        return Err(Error::other(format!("Cannot handle {} {}", field, size)));
    }
    Ok(size as usize)
}

#[inline]
fn read_u64le<R: Read>(reader: &mut R) -> Result<u64, Error> {
    let mut buf = [0; 8];
    reader.read_exact(&mut buf).map_err(Error::io)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64, Error> {
    let first = read_u8(reader)? as u64;
    let mut mask = 0x80 as u64;
    let mut value = 0;
    for i in 0..8 {
        if (first & mask) == 0 {
            return Ok(value | ((first & (mask - 1)) << (8 * i)));
        }
        let b = read_u8(reader)? as u64;
        value |= b << (8 * i);
        mask >>= 1;
    }
    Ok(value)
}

#[inline(always)]
fn read_u32<R: Read>(reader: &mut R) -> Result<u32, Error> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf).map_err(Error::io)?;
    Ok(u32::from_le_bytes(buf))
}

#[inline(always)]
fn read_u8<R: Read>(reader: &mut R) -> Result<u8, Error> {
    let mut buf = [0];
    reader.read_exact(&mut buf).map_err(Error::io)?;
    Ok(buf[0])
}

fn read_all_or_bits<R: Read>(header: &mut R, size: usize) -> Result<BitSet, Error> {
    let all = read_u8(header)?;
    if all != 0 {
        let mut bits = BitSet::with_capacity(size);
        for i in 0..size {
            bits.insert(i);
        }
        Ok(bits)
    } else {
        read_bits(header, size)
    }
}

fn read_bits<R: Read>(header: &mut R, size: usize) -> Result<BitSet, Error> {
    let mut bits = BitSet::with_capacity(size);
    let mut mask = 0u32;
    let mut cache = 0u32;
    for i in 0..size {
        if mask == 0 {
            mask = 0x80;
            cache = read_u8(header)? as u32;
        }
        if (cache & mask) != 0 {
            bits.insert(i);
        }
        mask = mask >> 1;
    }
    Ok(bits)
}

struct NamesReader<'a, R: Read> {
    max_bytes: usize,
    read_bytes: usize,
    cache: Vec<u16>,
    reader: &'a mut R,
}

impl<'a, R: Read> NamesReader<'a, R> {
    fn new(reader: &'a mut R, max_bytes: usize) -> Self {
        Self {
            max_bytes,
            reader,
            read_bytes: 0,
            cache: Vec::with_capacity(16),
        }
    }
}

impl<'a, R: Read> Iterator for NamesReader<'a, R> {
    type Item = Result<String, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.max_bytes <= self.read_bytes {
            return None;
        }
        self.cache.clear();
        let mut buf = [0; 2];
        while self.read_bytes < self.max_bytes {
            let r = self.reader.read_exact(&mut buf).map_err(Error::io);
            self.read_bytes += 2;
            if let Err(e) = r {
                return Some(Err(e));
            }
            let u = u16::from_le_bytes(buf);
            if u == 0 {
                break;
            }
            self.cache.push(u);
        }

        Some(String::from_utf16(&self.cache).map_err(|e| Error::other(e.to_string())))
    }
}

pub struct SevenZReader<R: Read + Seek> {
    source: R,
    archive: Archive,
    password: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SevenZReader<File> {
    #[inline]
    pub fn open(path: impl AsRef<std::path::Path>, password: Password) -> Result<Self, Error> {
        let file = std::fs::File::open(path.as_ref())
            .map_err(|e| Error::file_open(e, path.as_ref().to_string_lossy().to_string()))?;
        let len = file.metadata().map(|m| m.len()).map_err(Error::io)?;
        Self::new(file, len, password)
    }
}

impl<R: Read + Seek> SevenZReader<R> {
    #[inline]
    pub fn new(mut source: R, reader_len: u64, password: Password) -> Result<Self, Error> {
        let password = password.to_vec();
        let archive = Archive::read(&mut source, reader_len, &password)?;
        Ok(Self {
            source,
            archive,
            password,
        })
    }

    #[inline]
    pub fn from_archive(archive: Archive, source: R, password: Password) -> Self {
        Self {
            source,
            archive,
            password: password.to_vec(),
        }
    }

    #[inline]
    pub fn archive(&self) -> &Archive {
        &self.archive
    }

    fn build_decode_stack<'r>(
        source: &'r mut R,
        archive: &'r Archive,
        folder_index: usize,
        password: &[u8],
    ) -> Result<(Box<dyn Read + 'r>, usize), Error> {
        let folder = &archive.folders[folder_index];
        if folder.total_input_streams > folder.total_output_streams {
            return Self::build_decode_stack2(source, archive, folder_index, password);
        }
        let first_pack_stream_index =
            archive.stream_map.folder_first_pack_stream_index[folder_index];
        let folder_offset = SIGNATURE_HEADER_SIZE
            + archive.pack_pos
            + archive.stream_map.pack_stream_offsets[first_pack_stream_index];

        source
            .seek(SeekFrom::Start(folder_offset))
            .map_err(Error::io)?;
        let pack_size = archive.pack_sizes[first_pack_stream_index] as usize;

        let mut decoder: Box<dyn Read> = Box::new(BoundedReader::new(source, pack_size));
        let folder = &archive.folders[folder_index];
        for (index, coder) in folder.ordered_coder_iter() {
            if coder.num_in_streams != 1 || coder.num_out_streams != 1 {
                return Err(Error::unsupported(
                    "Multi input/output stream coders are not yet supported",
                ));
            }
            let next = crate::decoders::add_decoder(
                decoder,
                folder.get_unpack_size_at_index(index) as usize,
                coder,
                password,
                MAX_MEM_LIMIT_KB,
            )?;
            decoder = Box::new(next);
        }
        if folder.has_crc {
            decoder = Box::new(Crc32VerifyingReader::new(
                decoder,
                folder.get_unpack_size() as usize,
                folder.crc,
            ));
        }

        Ok((decoder, pack_size))
    }

    fn build_decode_stack2<'r>(
        source: &'r mut R,
        archive: &'r Archive,
        folder_index: usize,
        password: &[u8],
    ) -> Result<(Box<dyn Read + 'r>, usize), Error> {
        const MAX_CODER_COUNT: usize = 32;
        let folder = &archive.folders[folder_index];
        if folder.coders.len() > MAX_CODER_COUNT {
            return Err(Error::unsupported(format!(
                "Too many coders: {}",
                folder.coders.len()
            )));
        }

        assert!(folder.total_input_streams > folder.total_output_streams);
        let source = ReaderPtr::new(source);
        let first_pack_stream_index =
            archive.stream_map.folder_first_pack_stream_index[folder_index];
        let start_pos = SIGNATURE_HEADER_SIZE + archive.pack_pos;
        let offsets = &archive.stream_map.pack_stream_offsets[first_pack_stream_index..];

        let mut sources = Vec::with_capacity(folder.packed_streams.len());
        for i in 0..folder.packed_streams.len() {
            let pack_pos = start_pos + offsets[i];
            let pack_size = archive.pack_sizes[first_pack_stream_index + i];
            let pack_reader =
                SeekableBoundedReader::new(source.clone(), (pack_pos, pack_pos + pack_size));
            sources.push(pack_reader);
        }

        let mut coder_to_stream_map = [usize::MAX; MAX_CODER_COUNT];

        let mut si = 0;
        for i in 0..folder.coders.len() {
            coder_to_stream_map[i] = si;
            si += folder.coders[i].num_in_streams as usize;
        }

        let main_coder_index = {
            let mut coder_used = [false; MAX_CODER_COUNT];
            for bp in folder.bind_pairs.iter() {
                coder_used[bp.out_index as usize] = true;
            }
            let mut mci = 0;
            for i in 0..folder.coders.len() {
                if !coder_used[i] {
                    mci = i;
                    break;
                }
            }
            mci
        };

        let id = folder.coders[main_coder_index].decompression_method_id();
        if id != SevenZMethod::ID_BCJ2 {
            return Err(Error::unsupported(format!("Unsupported method: {:?}", id)));
        }

        let num_in_streams = folder.coders[main_coder_index].num_in_streams as usize;
        let mut inputs: Vec<Box<dyn Read>> = Vec::with_capacity(num_in_streams);
        let start_i = coder_to_stream_map[main_coder_index];
        for i in start_i..num_in_streams + start_i {
            inputs.push(Self::get_in_stream(
                folder,
                &sources,
                &coder_to_stream_map,
                password,
                i,
            )?);
        }
        let mut decoder: Box<dyn Read> = Box::new(crate::bcj2::BCJ2Reader::new(
            inputs,
            folder.get_unpack_size(),
        ));
        if folder.has_crc {
            decoder = Box::new(Crc32VerifyingReader::new(
                decoder,
                folder.get_unpack_size() as usize,
                folder.crc,
            ));
        }
        Ok((
            decoder,
            archive.pack_sizes[first_pack_stream_index] as usize,
        ))
    }

    fn get_in_stream<'r>(
        folder: &Folder,
        sources: &[SeekableBoundedReader<ReaderPtr<R>>],
        coder_to_stream_map: &[usize],
        password: &[u8],

        in_stream_index: usize,
    ) -> Result<Box<dyn Read + 'r>, Error>
    where
        R: 'r,
    {
        let index = folder
            .packed_streams
            .iter()
            .position(|&i| i == in_stream_index as u64);
        if let Some(index) = index {
            return Ok(Box::new(sources[index as usize].clone()));
        }

        let bp = folder
            .find_bind_pair_for_in_stream(in_stream_index)
            .ok_or_else(|| {
                Error::other(format!(
                    "Couldn't find bind pair for stream {}",
                    in_stream_index
                ))
            })?;
        let index = folder.bind_pairs[bp].out_index as usize;

        Self::get_in_stream2(folder, sources, coder_to_stream_map, password, index)
    }

    fn get_in_stream2<'r>(
        folder: &Folder,
        sources: &[SeekableBoundedReader<ReaderPtr<R>>],
        coder_to_stream_map: &[usize],
        password: &[u8],
        in_stream_index: usize,
    ) -> Result<Box<dyn Read + 'r>, Error>
    where
        R: 'r,
    {
        let coder = &folder.coders[in_stream_index];
        let start_index = coder_to_stream_map[in_stream_index];
        if start_index == usize::MAX {
            return Err(Error::other("in_stream_index out of range"));
        }
        let uncompressed_len = folder.unpack_sizes[in_stream_index] as usize;
        if coder.num_in_streams == 1 {
            let input =
                Self::get_in_stream(folder, sources, coder_to_stream_map, password, start_index)?;

            let decoder = add_decoder(input, uncompressed_len, coder, password, MAX_MEM_LIMIT_KB)?;
            return Ok(Box::new(decoder));
        }
        return Err(Error::unsupported(
            "Multi input stream coders are not yet supported",
        ));
    }

    pub fn for_each_entries<F: FnMut(&SevenZArchiveEntry, &mut dyn Read) -> Result<bool, Error>>(
        &mut self,
        mut each: F,
    ) -> Result<(), Error> {
        let folder_count = self.archive.folders.len();
        for folder_index in 0..folder_count {
            let forder_dec = FolderDecoder::new(
                folder_index,
                &self.archive,
                &self.password,
                &mut self.source,
            );
            forder_dec.for_each_entries(&mut each)?;
        }
        // decode empty files
        for file_index in 0..self.archive.files.len() {
            let folder_index = self.archive.stream_map.file_folder_index[file_index];
            if folder_index.is_none() {
                let file = &self.archive.files[file_index];
                let empty_reader: &mut dyn Read = &mut ([0u8; 0].as_slice());
                if !each(file, empty_reader)? {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

pub struct FolderDecoder<'a, R: Read + Seek> {
    folder_index: usize,
    archive: &'a Archive,
    password: &'a [u8],
    source: &'a mut R,
}

impl<'a, R: Read + Seek> FolderDecoder<'a, R> {
    pub fn new(
        folder_index: usize,
        archive: &'a Archive,
        password: &'a [u8],
        source: &'a mut R,
    ) -> Self {
        Self {
            folder_index,
            archive,
            password,
            source,
        }
    }

    pub fn entries(&self) -> &[SevenZArchiveEntry] {
        let start = self.archive.stream_map.folder_first_file_index[self.folder_index];
        let file_count = self.archive.folders[self.folder_index].num_unpack_sub_streams;
        &self.archive.files[start..(file_count + start)]
    }

    pub fn entry_count(&self) -> usize {
        self.archive.folders[self.folder_index].num_unpack_sub_streams
    }

    pub fn for_each_entries<F: FnMut(&SevenZArchiveEntry, &mut dyn Read) -> Result<bool, Error>>(
        self,
        each: &mut F,
    ) -> Result<bool, Error> {
        let Self {
            folder_index,
            archive,
            password,
            source,
        } = self;
        let (mut folder_reader, _size) =
            SevenZReader::build_decode_stack(source, archive, folder_index, password)?;
        let start = archive.stream_map.folder_first_file_index[folder_index];
        let file_count = archive.folders[folder_index].num_unpack_sub_streams;

        for file_index in start..(file_count + start) {
            let file = &archive.files[file_index];
            if file.has_stream && file.size > 0 {
                let mut decoder: Box<dyn Read> =
                    Box::new(BoundedReader::new(&mut folder_reader, file.size as usize));
                if file.has_crc {
                    decoder = Box::new(Crc32VerifyingReader::new(
                        decoder,
                        file.size as usize,
                        file.crc,
                    ));
                }
                if !each(file, &mut decoder)? {
                    return Ok(false);
                }
            } else {
                let empty_reader: &mut dyn Read = &mut ([0u8; 0].as_slice());
                if !each(file, empty_reader)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

#[derive(Debug, Copy)]
struct ReaderPtr<R> {
    reader: *mut R,
}

impl<R> Clone for ReaderPtr<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader,
        }
    }
}

impl<R> ReaderPtr<R> {
    fn new(reader: &mut R) -> Self {
        Self {
            reader: reader as *mut R,
        }
    }
}

impl<R: Read> Read for ReaderPtr<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe { (*self.reader).read(buf) }
    }
}

impl<R: Seek> Seek for ReaderPtr<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        unsafe { (*self.reader).seek(pos) }
    }
}
