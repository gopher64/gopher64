use byteorder::{LittleEndian, WriteBytesExt};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Password(Vec<u8>);

impl Password {
    pub fn empty() -> Self {
        Self(Default::default())
    }
    pub fn to_vec(self) -> Vec<u8> {
        self.0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
impl AsRef<[u8]> for Password {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for Password {
    fn from(s: &str) -> Self {
        let mut result = Vec::with_capacity(s.len() * 2);
        let utf16 = s.encode_utf16();
        for u in utf16 {
            let _ = result.write_u16::<LittleEndian>(u);
        }
        Self(result)
    }
}

impl From<&[u16]> for Password {
    fn from(s: &[u16]) -> Self {
        let mut result = Vec::with_capacity(s.len() * 2);
        for u in s {
            let _ = result.write_u16::<LittleEndian>(*u);
        }
        Self(result)
    }
}
