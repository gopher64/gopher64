use core::ops::Deref;

use crate::{ffi, Block};

/// A reference-counted Objective-C block.
pub struct RcBlock<A, R> {
    pub(crate) ptr: *mut Block<A, R>,
}

impl<A, R> RcBlock<A, R> {
    /// Construct an `RcBlock` for the given block without copying it.
    /// The caller must ensure the block has a +1 reference count.
    ///
    /// # Safety
    ///
    /// The given pointer must point to a valid `Block` and must have a +1
    /// reference count or it will be overreleased when the `RcBlock` is
    /// dropped.
    pub unsafe fn new(ptr: *mut Block<A, R>) -> Self {
        RcBlock { ptr }
    }

    /// Constructs an `RcBlock` by copying the given block.
    ///
    /// # Safety
    ///
    /// The given pointer must point to a valid `Block`.
    pub unsafe fn copy(ptr: *mut Block<A, R>) -> Self {
        // SAFETY: The caller ensures the pointer is valid.
        let ptr: *mut Block<A, R> = unsafe { ffi::_Block_copy(ptr.cast()) }.cast();
        // SAFETY: We just copied the block, so the reference count is +1
        //
        // TODO: Does _Block_copy always returns a valid pointer?
        unsafe { Self::new(ptr) }
    }
}

impl<A, R> Clone for RcBlock<A, R> {
    fn clone(&self) -> RcBlock<A, R> {
        // SAFETY: The pointer is valid, since the only way to get an RcBlock
        // in the first place is through unsafe functions.
        unsafe { RcBlock::copy(self.ptr) }
    }
}

impl<A, R> Deref for RcBlock<A, R> {
    type Target = Block<A, R>;

    fn deref(&self) -> &Block<A, R> {
        // SAFETY: The pointer is ensured valid by creator functions.
        unsafe { self.ptr.as_ref().unwrap_unchecked() }
    }
}

impl<A, R> Drop for RcBlock<A, R> {
    fn drop(&mut self) {
        unsafe { ffi::_Block_release(self.ptr.cast()) };
    }
}
