//! Tests macros in a hostile environment (`#![no_implicit_prelude]` and all
//! functions, modules, traits, and types replaced with custom bogus user
//! replacements).
//!
//! Heavy inspiration for this file taken from `objrs`:
//! https://gitlab.com/objrs/objrs/-/blob/b4f6598696b3fa622e6fddce7aff281770b0a8c2/test/src/no_prelude.rs
#![no_implicit_prelude]
#![allow(dead_code, non_camel_case_types)]

extern crate objc2 as new_objc2;

#[cfg(feature = "gnustep-1-7")]
#[test]
fn ensure_linkage() {
    unsafe { new_objc2::__gnustep_hack::get_class_to_force_linkage() };
}

mod core {}
mod std {}
mod libc {}
mod objc2 {}

enum BogusType {}
type u8 = BogusType;
type u16 = BogusType;
type u32 = BogusType;
type u64 = BogusType;
type u128 = BogusType;
type usize = BogusType;

type i8 = BogusType;
type i16 = BogusType;
type i32 = BogusType;
type i64 = BogusType;
type i128 = BogusType;
type isize = BogusType;

type bool = BogusType;
type char = BogusType;
type str = BogusType;

type f32 = BogusType;
type f64 = BogusType;

type Option = BogusType;
type Some = BogusType;
type None = BogusType;

type Result = BogusType;
type Ok = BogusType;
type Err = BogusType;

type Box = BogusType;
type String = BogusType;
type Vec = BogusType;

type drop = BogusType;

type Copy = BogusType;
type Send = BogusType;
type Sized = BogusType;
type Sync = BogusType;
type Drop = BogusType;
type Fn = BogusType;
type FnMut = BogusType;
type FnOnce = BogusType;

type ToOwned = BogusType;
type Clone = BogusType;
type PartialEq = BogusType;
type PartialOrd = BogusType;
type Eq = BogusType;
type Ord = BogusType;
type AsRef = BogusType;
type AsMut = BogusType;
type Into = BogusType;
type From = BogusType;
type Default = BogusType;
type Hash = BogusType;
type Debug = BogusType;
type Iterator = BogusType;
type Extend = BogusType;
type IntoIterator = BogusType;
type DoubleEndedIterator = BogusType;
type ExactSizeIterator = BogusType;
type SliceConcatExt = BogusType;
type ToString = BogusType;

pub fn test_selector() {
    let _sel = new_objc2::sel!(abc);
    let _sel = new_objc2::sel!(abc:def:);
}

pub fn test_class() {
    let _class = new_objc2::class!(NSObject);
}

pub fn test_msg_send(obj: &new_objc2::foundation::NSString) {
    let superclass = obj.class().superclass().unwrap();
    let _: () = unsafe { new_objc2::msg_send![obj, a] };
    let _: () = unsafe { new_objc2::msg_send![obj, a: obj, b: obj] };
    let _: () = unsafe { new_objc2::msg_send![super(obj), a] };
    let _: () = unsafe { new_objc2::msg_send![super(obj), a: obj, b: obj] };
    let _: () = unsafe { new_objc2::msg_send![super(obj, superclass), a] };
    let _: () = unsafe { new_objc2::msg_send![super(obj, superclass), a: obj, b: obj] };
}

pub fn test_msg_send_id(obj: &new_objc2::runtime::Object) {
    let _: new_objc2::rc::Id<new_objc2::runtime::Object, new_objc2::rc::Shared> =
        unsafe { new_objc2::msg_send_id![obj, a] };
    let _: new_objc2::__macro_helpers::Option<
        new_objc2::rc::Id<new_objc2::runtime::Object, new_objc2::rc::Shared>,
    > = unsafe { new_objc2::msg_send_id![obj, a] };
    let _: new_objc2::rc::Id<new_objc2::runtime::Object, new_objc2::rc::Shared> =
        unsafe { new_objc2::msg_send_id![obj, a: obj, b: obj] };
}
