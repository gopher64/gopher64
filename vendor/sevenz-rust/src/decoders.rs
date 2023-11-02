use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
#[cfg(feature = "bzip2")]
use bzip2::read::BzDecoder;

#[cfg(feature = "aes256")]
use crate::aes256sha256::Aes256Sha256Decoder;
use crate::{
    archive::SevenZMethod,
    bcj::SimpleReader,
    delta::DeltaReader,
    error::Error,
    folder::Coder,
    lzma::{lzma2_get_memery_usage, LZMA2Reader, LZMAReader},
};

pub enum Decoder<R: Read> {
    COPY(R),
    LZMA(LZMAReader<R>),
    LZMA2(LZMA2Reader<R>),
    BCJ(SimpleReader<R>),
    Delta(DeltaReader<R>),
    #[cfg(feature = "zstd")]
    ZSTD(zstd::Decoder<'static, std::io::BufReader<R>>),
    #[cfg(feature = "bzip2")]
    BZip2(BzDecoder<R>),
    #[cfg(feature = "aes256")]
    AES256SHA256(Aes256Sha256Decoder<R>),
}

// impl<R: Read> Decoder<R> {
//     pub fn num_streams(&self) -> usize {
//         match self {
//             Self::BCJ(_) => 4,
//             _ => 1,
//         }
//     }
// }

impl<R: Read> Read for Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(feature = "zstd")]
            Decoder::ZSTD(r) => r.read(buf),
            Decoder::COPY(r) => r.read(buf),
            Decoder::LZMA(r) => r.read(buf),
            Decoder::LZMA2(r) => r.read(buf),
            Decoder::BCJ(r) => r.read(buf),
            Decoder::Delta(r) => r.read(buf),
            #[cfg(feature = "bzip2")]
            Decoder::BZip2(r) => r.read(buf),
            #[cfg(feature = "aes256")]
            Decoder::AES256SHA256(r) => r.read(buf),
        }
    }
}

pub fn add_decoder<I: Read>(
    input: I,
    uncompressed_len: usize,
    coder: &Coder,
    #[allow(unused)] password: &[u8],
    max_mem_limit_kb: usize,
) -> Result<Decoder<I>, Error> {
    let method = SevenZMethod::by_id(coder.decompression_method_id());
    let method = if let Some(m) = method {
        m
    } else {
        return Err(Error::UnsupportedCompressionMethod(format!(
            "{:?}",
            coder.decompression_method_id()
        )));
    };
    match method.id() {
        SevenZMethod::ID_COPY => Ok(Decoder::COPY(input)),
        #[cfg(feature = "zstd")]
        SevenZMethod::ID_ZSTD => {
            let props = coder.properties[0];
            let zs = zstd::Decoder::new(input).unwrap();
            Ok(Decoder::ZSTD(zs))
        }
        SevenZMethod::ID_LZMA => {
            let dict_size = get_lzma_dic_size(coder)?;
            if coder.properties.len() < 1 {
                return Err(Error::Other("LZMA properties too short".into()));
            }
            let props = coder.properties[0];
            let lz =
                LZMAReader::new_with_props(input, uncompressed_len as _, props, dict_size, None)
                    .map_err(|e| Error::io(e))?;
            Ok(Decoder::LZMA(lz))
        }
        SevenZMethod::ID_LZMA2 => {
            let dic_size = get_lzma2_dic_size(coder)?;
            let mem_size = lzma2_get_memery_usage(dic_size) as usize;
            if mem_size > max_mem_limit_kb {
                return Err(Error::MaxMemLimited {
                    max_kb: max_mem_limit_kb,
                    actaul_kb: mem_size,
                });
            }
            let lz = LZMA2Reader::new(input, dic_size, None);
            Ok(Decoder::LZMA2(lz))
        }
        SevenZMethod::ID_BCJ_X86 => {
            let de = SimpleReader::new_x86(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_ARM => {
            let de = SimpleReader::new_arm(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_ARM_THUMB => {
            let de = SimpleReader::new_arm_thumb(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_PPC => {
            let de = SimpleReader::new_ppc(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_BCJ_SPARC => {
            let de = SimpleReader::new_sparc(input);
            Ok(Decoder::BCJ(de))
        }
        SevenZMethod::ID_DELTA => {
            let d = if coder.properties.is_empty() {
                1
            } else {
                (coder.properties[0] & 0xff) + 1
            };
            let de = DeltaReader::new(input, d as usize);
            Ok(Decoder::Delta(de))
        }
        #[cfg(feature = "bzip2")]
        SevenZMethod::ID_BZIP2 => {
            let de = BzDecoder::new(input);
            Ok(Decoder::BZip2(de))
        }
        #[cfg(feature = "aes256")]
        SevenZMethod::ID_AES256SHA256 => {
            let de = Aes256Sha256Decoder::new(input, coder, password)?;
            Ok(Decoder::AES256SHA256(de))
        }
        _ => {
            return Err(Error::UnsupportedCompressionMethod(
                method.name().to_string(),
            ));
        }
    }
}

#[inline]
fn get_lzma2_dic_size(coder: &Coder) -> Result<u32, Error> {
    if coder.properties.len() < 1 {
        return Err(Error::other("LZMA2 properties too short"));
    }
    let dict_size_bits = 0xff & coder.properties[0] as u32;
    if (dict_size_bits & (!0x3f)) != 0 {
        return Err(Error::other("Unsupported LZMA2 property bits"));
    }
    if dict_size_bits > 40 {
        return Err(Error::other("Dictionary larger than 4GiB maximum size"));
    }
    if dict_size_bits == 40 {
        return Ok(0xFFFFffff);
    }
    let size = (2 | (dict_size_bits & 0x1)) << (dict_size_bits / 2 + 11);
    Ok(size)
}

#[inline]
fn get_lzma_dic_size(coder: &Coder) -> Result<u32, Error> {
    let mut props = &coder.properties[1..5];
    props.read_u32::<LittleEndian>().map_err(|e| Error::io(e))
}
