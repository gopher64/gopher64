use crate::preparse::{FaceSubtables, PreParsedSubtables};
#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};
use core::{fmt, marker::PhantomPinned, pin::Pin, slice};

/// An owned version of font [`Face`](struct.Face.html).
pub struct OwnedFace(Pin<Box<SelfRefVecFace>>);

impl OwnedFace {
    /// Creates an `OwnedFace` from owned data.
    ///
    /// You can set index for font collections. For simple ttf fonts set index to 0.
    ///
    /// # Example
    /// ```
    /// # use owned_ttf_parser::OwnedFace;
    /// # let owned_font_data = include_bytes!("../fonts/font.ttf").to_vec();
    /// let owned_face = OwnedFace::from_vec(owned_font_data, 0).unwrap();
    /// ```
    // Note: not `try_from_vec` to better mimic `ttf_parser::Face::from_data`.
    pub fn from_vec(data: Vec<u8>, index: u32) -> Result<Self, ttf_parser::FaceParsingError> {
        let inner = SelfRefVecFace::try_from_vec(data, index)?;
        Ok(Self(inner))
    }

    pub(crate) fn pre_parse_subtables(self) -> PreParsedSubtables<'static, Self> {
        // build subtables referencing fake static data
        let subtables = FaceSubtables::from(match self.0.face.as_ref() {
            Some(f) => f,
            None => unsafe { core::hint::unreachable_unchecked() },
        });

        // bundle everything together so self-reference lifetimes hold
        PreParsedSubtables {
            face: self,
            subtables,
        }
    }

    /// Extracts a slice containing the data passed into [`OwnedFace::from_vec`].
    ///
    /// # Example
    /// ```
    /// # use owned_ttf_parser::OwnedFace;
    /// # let owned_font_data = include_bytes!("../fonts/font.ttf").to_vec();
    /// let data_clone = owned_font_data.clone();
    /// let owned_face = OwnedFace::from_vec(owned_font_data, 0).unwrap();
    /// assert_eq!(owned_face.as_slice(), data_clone);
    /// ```
    pub fn as_slice(&self) -> &[u8] {
        &self.0.data
    }

    /// Unwraps the data passed into [`OwnedFace::from_vec`].
    ///
    /// # Example
    /// ```
    /// # use owned_ttf_parser::OwnedFace;
    /// # let owned_font_data = include_bytes!("../fonts/font.ttf").to_vec();
    /// let data_clone = owned_font_data.clone();
    /// let owned_face = OwnedFace::from_vec(owned_font_data, 0).unwrap();
    /// assert_eq!(owned_face.into_vec(), data_clone);
    /// ```
    pub fn into_vec(self) -> Vec<u8> {
        // safe as the `Face` is dropped.
        unsafe { Pin::into_inner_unchecked(self.0).data }
    }
}

impl fmt::Debug for OwnedFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OwnedFace()")
    }
}

impl crate::convert::AsFaceRef for OwnedFace {
    #[inline]
    fn as_face_ref(&self) -> &ttf_parser::Face<'_> {
        self.0.inner_ref()
    }
}

impl crate::convert::AsFaceRef for &OwnedFace {
    #[inline]
    fn as_face_ref(&self) -> &ttf_parser::Face<'_> {
        self.0.inner_ref()
    }
}

impl crate::convert::FaceMut for OwnedFace {
    fn set_variation(&mut self, axis: ttf_parser::Tag, value: f32) -> Option<()> {
        unsafe {
            let mut_ref = Pin::as_mut(&mut self.0);
            let mut_inner = mut_ref.get_unchecked_mut();
            match mut_inner.face.as_mut() {
                Some(face) => face.set_variation(axis, value),
                None => None,
            }
        }
    }
}
impl crate::convert::FaceMut for &mut OwnedFace {
    #[inline]
    fn set_variation(&mut self, axis: ttf_parser::Tag, value: f32) -> Option<()> {
        (*self).set_variation(axis, value)
    }
}

// Face data in a `Vec` with a self-referencing `Face`.
struct SelfRefVecFace {
    data: Vec<u8>,
    face: Option<ttf_parser::Face<'static>>,
    _pin: PhantomPinned,
}

impl SelfRefVecFace {
    /// Creates an underlying face object from owned data.
    fn try_from_vec(
        data: Vec<u8>,
        index: u32,
    ) -> Result<Pin<Box<Self>>, ttf_parser::FaceParsingError> {
        let face = Self {
            data,
            face: None,
            _pin: PhantomPinned,
        };
        let mut b = Box::pin(face);
        unsafe {
            // 'static lifetime is a lie, this data is owned, it has pseudo-self lifetime.
            let slice: &'static [u8] = slice::from_raw_parts(b.data.as_ptr(), b.data.len());
            let mut_ref: Pin<&mut Self> = Pin::as_mut(&mut b);
            let mut_inner = mut_ref.get_unchecked_mut();
            mut_inner.face = Some(ttf_parser::Face::from_slice(slice, index)?);
        }
        Ok(b)
    }

    // Must not leak the fake 'static lifetime that we lied about earlier to the
    // compiler. Since the lifetime 'a will not outlive our owned data it's
    // safe to provide Face<'a>
    #[inline]
    #[allow(clippy::needless_lifetimes)] // explicit is nice as it's important 'static isn't leaked
    fn inner_ref<'a>(self: &'a Pin<Box<Self>>) -> &'a ttf_parser::Face<'a> {
        match self.face.as_ref() {
            Some(f) => f,
            None => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}
