//! A custom Objective-C class with a lifetime parameter.
//!
//! Note that we can't use the `declare_class!` macro for this, it doesn't
//! support such use-cases. Instead, we'll declare the class manually!
#![deny(unsafe_op_in_unsafe_fn)]
use std::marker::PhantomData;
use std::sync::Once;

use objc2::declare::{ClassBuilder, Ivar, IvarType};
use objc2::foundation::NSObject;
use objc2::rc::{Id, Owned};
use objc2::runtime::{Class, Object, Sel};
use objc2::{msg_send, msg_send_id, sel};
use objc2::{ClassType, Encoding, Message, RefEncode};

/// Helper type for the instance variable
struct NumberIvar<'a> {
    // Doesn't actually matter what we put here, but we have to use the
    // lifetime parameter somehow
    p: PhantomData<&'a mut u8>,
}

unsafe impl<'a> IvarType for NumberIvar<'a> {
    type Type = &'a mut u8;
    const NAME: &'static str = "_number_ptr";
}

/// Struct that represents our custom object.
#[repr(C)]
pub struct MyObject<'a> {
    // Required to give MyObject the proper layout
    superclass: NSObject,
    // SAFETY: The ivar is declared below, and is properly initialized in the
    // designated initializer.
    //
    // Note! Attempting to acess the ivar before it has been initialized is
    // undefined behaviour!
    number: Ivar<NumberIvar<'a>>,
}

unsafe impl RefEncode for MyObject<'_> {
    const ENCODING_REF: Encoding = Object::ENCODING_REF;
}

unsafe impl Message for MyObject<'_> {}

impl<'a> MyObject<'a> {
    unsafe extern "C" fn init_with_ptr(
        &mut self,
        _cmd: Sel,
        ptr: Option<&'a mut u8>,
    ) -> Option<&'a mut Self> {
        let this: Option<&mut Self> = unsafe { msg_send![super(self), init] };
        this.map(|this| {
            // Properly initialize the number reference
            Ivar::write(&mut this.number, ptr.expect("got NULL number ptr"));
            this
        })
    }

    pub fn new(number: &'a mut u8) -> Id<Self, Owned> {
        // SAFETY: The lifetime of the reference is properly bound to the
        // returned type
        unsafe {
            let obj = msg_send_id![Self::class(), alloc];
            msg_send_id![obj, initWithPtr: number]
        }
    }

    pub fn get(&self) -> &u8 {
        &self.number
    }

    pub fn set(&mut self, number: u8) {
        **self.number = number;
    }
}

unsafe impl<'a> ClassType for MyObject<'a> {
    type Super = NSObject;
    const NAME: &'static str = "MyObject";

    fn class() -> &'static Class {
        // TODO: Use std::lazy::LazyCell
        static REGISTER_CLASS: Once = Once::new();

        REGISTER_CLASS.call_once(|| {
            let superclass = NSObject::class();
            let mut builder = ClassBuilder::new(Self::NAME, superclass).unwrap();

            builder.add_static_ivar::<NumberIvar<'a>>();

            unsafe {
                builder.add_method(
                    sel!(initWithPtr:),
                    Self::init_with_ptr as unsafe extern "C" fn(_, _, _) -> _,
                );
            }

            let _cls = builder.register();
        });

        Class::get("MyObject").unwrap()
    }

    fn as_super(&self) -> &Self::Super {
        &self.superclass
    }

    fn as_super_mut(&mut self) -> &mut Self::Super {
        &mut self.superclass
    }
}

fn main() {
    let mut number = 54;
    let mut obj = MyObject::new(&mut number);

    // It is not possible to convert to `Id<NSObject, Owned>` since that would
    // loose the lifetime information that `MyObject` stores
    // let obj = Id::into_super(obj);

    println!("Number: {}", obj.get());

    obj.set(7);
    // Won't compile, since `obj` holds a mutable reference to number
    // println!("Number: {}", number);
    println!("Number: {}", obj.get());

    let obj = Id::into_shared(obj);
    let obj2 = obj.clone();

    // We gave up ownership above, so can't edit the number any more!
    // obj.set(7);

    println!("Number: {}", obj.get());
    println!("Number: {}", obj2.get());

    drop(obj);
    drop(obj2);
    println!("Number: {}", number);
}
