use objc2::foundation::NSString;
use objc2::runtime::{Class, Object};
use objc2::{class, msg_send, sel};

#[cfg(feature = "gnustep-1-7")]
#[test]
fn ensure_linkage() {
    unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
}

#[test]
fn use_class_and_msg_send() {
    unsafe {
        let cls = class!(NSObject);
        let obj: *mut Object = msg_send![cls, new];
        let _hash: usize = msg_send![obj, hash];
        let _: () = msg_send![obj, release];
    }
}

#[test]
fn use_sel() {
    let _sel = sel!(description);
    let _sel = sel!(setObject:forKey:);
}

#[allow(unused)]
fn test_msg_send_comma_handling(obj: &NSString, superclass: &Class) {
    unsafe {
        let _: () = msg_send![obj, a];
        let _: () = msg_send![obj, a,];
        let _: () = msg_send![obj, a: 32i32];
        let _: () = msg_send![obj, a: 32i32,];
        let _: () = msg_send![obj, a: 32i32 b: 32i32];
        let _: () = msg_send![obj, a: 32i32 b: 32i32,];
        let _: () = msg_send![obj, a: 32i32, b: 32i32];
        let _: () = msg_send![obj, a: 32i32, b: 32i32,];
    }

    unsafe {
        let _: () = msg_send![super(obj, superclass), a];
        let _: () = msg_send![super(obj, superclass), a,];
        let _: () = msg_send![super(obj, superclass), a: 32i32];
        let _: () = msg_send![super(obj, superclass), a: 32i32,];
        let _: () = msg_send![super(obj, superclass), a: 32i32 b: 32i32];
        let _: () = msg_send![super(obj, superclass), a: 32i32 b: 32i32,];
        let _: () = msg_send![super(obj, superclass), a: 32i32, b: 32i32];
        let _: () = msg_send![super(obj, superclass), a: 32i32, b: 32i32,];
    }

    unsafe {
        let _: () = msg_send![super(obj), a];
        let _: () = msg_send![super(obj), a,];
        let _: () = msg_send![super(obj), a: 32i32];
        let _: () = msg_send![super(obj), a: 32i32,];
        let _: () = msg_send![super(obj), a: 32i32, b: 32i32];
        let _: () = msg_send![super(obj), a: 32i32, b: 32i32,];
    }
}
