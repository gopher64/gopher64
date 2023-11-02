use core::cell::RefCell;
use core::mem::ManuallyDrop;
use core::ptr;

use super::{Id, Owned};
use crate::foundation::{NSObject, NSZone};
use crate::{declare_class, msg_send, ClassType};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ThreadTestData {
    pub(crate) alloc: usize,
    pub(crate) dealloc: usize,
    pub(crate) init: usize,
    pub(crate) retain: usize,
    pub(crate) copy: usize,
    pub(crate) mutable_copy: usize,
    pub(crate) release: usize,
    pub(crate) autorelease: usize,
    pub(crate) try_retain: usize,
    pub(crate) try_retain_fail: usize,
}

impl ThreadTestData {
    /// Get the amount of method calls performed on the current thread.
    pub(crate) fn current() -> ThreadTestData {
        TEST_DATA.with(|data| data.borrow().clone())
    }

    #[track_caller]
    pub(crate) fn assert_current(&self) {
        let current = Self::current();
        let mut expected = self.clone();
        if cfg!(feature = "gnustep-1-7") {
            // GNUStep doesn't have `tryRetain`, it uses `retain` directly
            let retain_diff = expected.try_retain - current.try_retain;
            expected.retain += retain_diff;
            expected.try_retain -= retain_diff;

            // GNUStep doesn't call `autorelease` if it's overridden
            expected.autorelease = 0;
        }
        assert_eq!(current, expected);
    }
}

std::thread_local! {
    pub(crate) static TEST_DATA: RefCell<ThreadTestData> = RefCell::new(Default::default());
}

declare_class!(
    /// A helper object that counts how many times various reference-counting
    /// primitives are called.
    #[derive(Debug, PartialEq)]
    pub(crate) struct RcTestObject {}

    unsafe impl ClassType for RcTestObject {
        type Super = NSObject;
    }

    unsafe impl RcTestObject {
        #[sel(newReturningNull)]
        fn new_returning_null() -> *mut Self {
            ptr::null_mut()
        }

        #[sel(alloc)]
        fn alloc() -> *mut Self {
            TEST_DATA.with(|data| data.borrow_mut().alloc += 1);
            let superclass = NSObject::class().metaclass();
            let zone: *const NSZone = ptr::null();
            unsafe { msg_send![super(Self::class(), superclass), allocWithZone: zone] }
        }

        #[sel(allocWithZone:)]
        fn alloc_with_zone(zone: *const NSZone) -> *mut Self {
            TEST_DATA.with(|data| data.borrow_mut().alloc += 1);
            let superclass = NSObject::class().metaclass();
            unsafe { msg_send![super(Self::class(), superclass), allocWithZone: zone] }
        }

        #[sel(allocReturningNull)]
        fn alloc_returning_null() -> *mut Self {
            ptr::null_mut()
        }

        #[sel(init)]
        fn init(&mut self) -> *mut Self {
            TEST_DATA.with(|data| data.borrow_mut().init += 1);
            unsafe { msg_send![super(self), init] }
        }

        #[sel(initReturningNull)]
        fn init_returning_null(&mut self) -> *mut Self {
            let _: () = unsafe { msg_send![self, release] };
            ptr::null_mut()
        }

        #[sel(retain)]
        fn retain(&self) -> *mut Self {
            TEST_DATA.with(|data| data.borrow_mut().retain += 1);
            unsafe { msg_send![super(self), retain] }
        }

        #[sel(release)]
        fn release(&self) {
            TEST_DATA.with(|data| data.borrow_mut().release += 1);
            unsafe { msg_send![super(self), release] }
        }

        #[sel(autorelease)]
        fn autorelease(&self) -> *mut Self {
            TEST_DATA.with(|data| data.borrow_mut().autorelease += 1);
            unsafe { msg_send![super(self), autorelease] }
        }

        #[sel(dealloc)]
        unsafe fn dealloc(&mut self) {
            TEST_DATA.with(|data| data.borrow_mut().dealloc += 1);
            unsafe { msg_send![super(self), dealloc] }
        }

        #[sel(_tryRetain)]
        unsafe fn try_retain(&self) -> bool {
            TEST_DATA.with(|data| data.borrow_mut().try_retain += 1);
            let res: bool = unsafe { msg_send![super(self), _tryRetain] };
            if !res {
                TEST_DATA.with(|data| data.borrow_mut().try_retain -= 1);
                TEST_DATA.with(|data| data.borrow_mut().try_retain_fail += 1);
            }
            res
        }

        #[sel(copyWithZone:)]
        fn copy_with_zone(&self, _zone: *const NSZone) -> *const Self {
            TEST_DATA.with(|data| data.borrow_mut().copy += 1);
            Id::consume_as_ptr(ManuallyDrop::new(Self::new()))
        }

        #[sel(mutableCopyWithZone:)]
        fn mutable_copy_with_zone(&self, _zone: *const NSZone) -> *const Self {
            TEST_DATA.with(|data| data.borrow_mut().mutable_copy += 1);
            Id::consume_as_ptr(ManuallyDrop::new(Self::new()))
        }

        #[sel(copyReturningNull)]
        fn copy_returning_null(&self) -> *const Self {
            ptr::null()
        }

        #[sel(methodReturningNull)]
        fn method_returning_null(&self) -> *const Self {
            ptr::null()
        }
    }
);

unsafe impl Send for RcTestObject {}
unsafe impl Sync for RcTestObject {}

impl RcTestObject {
    pub(crate) fn new() -> Id<Self, Owned> {
        // Use msg_send! - msg_send_id! is tested elsewhere!
        unsafe { Id::new(msg_send![Self::class(), new]) }.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_declared_name() {
        assert_eq!(RcTestObject::class().name(), RcTestObject::NAME);
    }
}
