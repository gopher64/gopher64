use core::hint;
use core::mem;

use super::conditional_try;
use crate::encode::Encode;
use crate::ffi;
use crate::runtime::{Class, Imp, Object, Sel};
use crate::MessageArguments;

#[inline]
fn unwrap_msg_send_fn(msg_send_fn: Option<Imp>) -> Imp {
    match msg_send_fn {
        Some(msg_send_fn) => msg_send_fn,
        None => {
            // SAFETY: This will never be NULL, even if the selector is not
            // found a callable function pointer will still be returned!
            //
            // `clang` doesn't insert a NULL check here either.
            unsafe { hint::unreachable_unchecked() }
        }
    }
}

#[track_caller]
pub(crate) unsafe fn send_unverified<A, R>(receiver: *mut Object, sel: Sel, args: A) -> R
where
    A: MessageArguments,
    R: Encode,
{
    // If `receiver` is NULL, objc_msg_lookup will return a standard C-method
    // taking two arguments, the receiver and the selector. Transmuting and
    // calling such a function with multiple parameters is UB, so instead we
    // just return NULL directly.
    if receiver.is_null() {
        // SAFETY: Caller guarantees that messages to NULL-receivers only
        // return pointers, and a mem::zeroed pointer is just a NULL-pointer.
        return unsafe { mem::zeroed() };
    }

    let msg_send_fn = unsafe { ffi::objc_msg_lookup(receiver.cast(), sel.as_ptr()) };
    let msg_send_fn = unwrap_msg_send_fn(msg_send_fn);
    unsafe { conditional_try(|| A::__invoke(msg_send_fn, receiver, sel, args)) }
}

#[track_caller]
pub(crate) unsafe fn send_super_unverified<A, R>(
    receiver: *mut Object,
    superclass: &Class,
    sel: Sel,
    args: A,
) -> R
where
    A: MessageArguments,
    R: Encode,
{
    if receiver.is_null() {
        // SAFETY: Same as in `send_unverified`.
        return unsafe { mem::zeroed() };
    }

    let superclass: *const Class = superclass;
    let sup = ffi::objc_super {
        receiver: receiver.cast(),
        super_class: superclass.cast(),
    };
    let msg_send_fn = unsafe { ffi::objc_msg_lookup_super(&sup, sel.as_ptr()) };
    let msg_send_fn = unwrap_msg_send_fn(msg_send_fn);
    unsafe { conditional_try(|| A::__invoke(msg_send_fn, receiver, sel, args)) }
}
