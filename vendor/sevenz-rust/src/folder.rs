#![allow(unused)]

#[derive(Debug, Default, Clone)]
pub struct Folder {
    pub coders: Vec<Coder>,
    pub total_input_streams: usize,
    pub total_output_streams: usize,
    pub bind_pairs: Vec<BindPair>,
    pub packed_streams: Vec<u64>,
    pub unpack_sizes: Vec<u64>,
    pub has_crc: bool,
    pub crc: u64,
    pub num_unpack_sub_streams: usize,
}

impl Folder {
    pub fn find_bind_pair_for_in_stream(&self, index: usize) -> Option<usize> {
        let index = index as u64;
        for i in 0..self.bind_pairs.len() {
            if self.bind_pairs[i].in_index == index {
                return Some(i);
            }
        }
        return None;
    }

    pub fn find_bind_pair_for_out_stream(&self, index: usize) -> Option<usize> {
        let index = index as u64;
        for i in 0..self.bind_pairs.len() {
            if self.bind_pairs[i].out_index == index {
                return Some(i);
            }
        }
        return None;
    }

    pub fn get_unpack_size(&self) -> u64 {
        if self.total_output_streams == 0 {
            return 0;
        }
        for i in (0..self.total_output_streams).rev() {
            if self.find_bind_pair_for_out_stream(i).is_none() {
                return self.unpack_sizes[i];
            }
        }
        return 0;
    }

    pub fn get_unpack_size_for_coder(&self, coder: &Coder) -> u64 {
        for i in 0..self.coders.len() {
            if std::ptr::eq(&self.coders[i], coder) {
                return self.unpack_sizes[i];
            }
        }
        0
    }

    pub fn get_unpack_size_at_index(&self, index: usize) -> u64 {
        self.unpack_sizes.get(index).cloned().unwrap_or_default()
    }

    pub fn ordered_coder_iter(&self) -> OrderedCoderIter {
        OrderedCoderIter::new(self)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Coder {
    decompression_method_id: [u8; 0xf],
    pub id_size: usize,
    pub num_in_streams: u64,
    pub num_out_streams: u64,
    pub properties: Vec<u8>,
}

impl Coder {
    pub fn decompression_method_id(&self) -> &[u8] {
        &self.decompression_method_id[0..self.id_size]
    }
    pub fn decompression_method_id_mut(&mut self) -> &mut [u8] {
        &mut self.decompression_method_id[0..self.id_size]
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BindPair {
    pub in_index: u64,
    pub out_index: u64,
}

pub struct OrderedCoderIter<'a> {
    folder: &'a Folder,
    current: Option<u64>,
}
impl<'a> OrderedCoderIter<'a> {
    fn new(folder: &'a Folder) -> Self {
        let current = folder.packed_streams.first().copied();
        Self { folder, current }
    }
}

impl<'a> Iterator for OrderedCoderIter<'a> {
    type Item = (usize, &'a Coder);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i) = self.current {
            self.current = if let Some(pair) = self.folder.find_bind_pair_for_out_stream(i as usize)
            {
                Some(self.folder.bind_pairs[pair].in_index)
            } else {
                None
            };
            self.folder
                .coders
                .get(i as usize)
                .map(|item| (i as usize, item))
        } else {
            None
        }
    }
}
