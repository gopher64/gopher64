use super::conditional_try;
use crate::encode::Encode;
use crate::ffi;
use crate::runtime::{Class, Imp, Object, Sel};
use crate::MessageArguments;

#[cfg(target_arch = "x86")]
#[path = "x86.rs"]
mod arch;
#[cfg(target_arch = "x86_64")]
#[path = "x86_64.rs"]
mod arch;
#[cfg(target_arch = "arm")]
#[path = "arm.rs"]
mod arch;
#[cfg(target_arch = "aarch64")]
#[path = "arm64.rs"]
mod arch;

/// On the above architectures we can statically find the correct method to
/// call from the return type, by looking at its `Encode` implementation.
#[allow(clippy::missing_safety_doc)]
unsafe trait MsgSendFn: Encode {
    const MSG_SEND: Imp;
    const MSG_SEND_SUPER: Imp;
}

#[inline]
#[track_caller]
pub(crate) unsafe fn send_unverified<A, R>(receiver: *mut Object, sel: Sel, args: A) -> R
where
    A: MessageArguments,
    R: Encode,
{
    let msg_send_fn = R::MSG_SEND;
    unsafe { conditional_try(|| A::__invoke(msg_send_fn, receiver, sel, args)) }
}

#[inline]
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
    let superclass: *const Class = superclass;
    let mut sup = ffi::objc_super {
        receiver: receiver.cast(),
        super_class: superclass.cast(),
    };
    let receiver: *mut ffi::objc_super = &mut sup;
    let receiver = receiver.cast();

    let msg_send_fn = R::MSG_SEND_SUPER;
    unsafe { conditional_try(|| A::__invoke(msg_send_fn, receiver, sel, args)) }
}
