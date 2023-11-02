use crate::GlyphId;
use alloc::boxed::Box;
use core::{fmt, iter};

pub struct CodepointIdIter<'a> {
    pub(crate) inner: Box<dyn Iterator<Item = (GlyphId, char)> + 'a>,
}

impl<'a> Iterator for CodepointIdIter<'a> {
    type Item = (GlyphId, char);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl iter::FusedIterator for CodepointIdIter<'_> {}

impl fmt::Debug for CodepointIdIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodepointIdIter")
    }
}
