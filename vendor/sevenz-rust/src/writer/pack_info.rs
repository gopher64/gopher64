use super::*;

#[derive(Debug, Default, Clone)]
pub struct PackInfo {
    pub crcs: Vec<u32>,
    pub sizes: Vec<u64>,
    pub pos: u64,
}

impl PackInfo {
    pub fn write_to<H: Write>(&mut self, header: &mut H) -> std::io::Result<()> {
        header.write_u8(K_PACK_INFO)?;
        write_u64(header, self.pos)?;
        write_u64(header, self.len() as u64)?;
        header.write_u8(K_SIZE)?;
        for size in &self.sizes {
            write_u64(header, *size)?;
        }
        header.write_u8(K_CRC)?;
        let all_crc_defined = self.crcs.iter().all(|f| *f != 0);
        if all_crc_defined {
            header.write_u8(1)?; // all defined
            for crc in self.crcs.iter() {
                header.write_u32::<LittleEndian>(*crc)?;
            }
        } else {
            header.write_u8(0)?; // not all defined
            let mut crc_define_bits = BitSet::with_capacity(self.crcs.len());

            let mut i = 0;
            for crc in self.crcs.iter().cloned() {
                if crc != 0 {
                    crc_define_bits.insert(i);
                }
                i += 1;
            }
            let mut temp = Vec::with_capacity(self.len());
            write_bit_set(&mut temp, &crc_define_bits)?;
            header.write_all(&temp)?;
        }

        header.write_u8(K_END)?;
        Ok(())
    }
}

impl PackInfo {
    #[inline]
    pub fn add_stream(&mut self, size: u64, crc: u32) {
        self.sizes.push(size);
        self.crcs.push(crc);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.sizes.len()
    }
}
