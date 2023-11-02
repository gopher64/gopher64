use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use std::os::raw::c_char;
use std::sync::Once;

use crate::declare::{ClassBuilder, ProtocolBuilder};
use crate::runtime::{Class, Object, Protocol, Sel};
use crate::{ffi, Encode, Encoding, MessageReceiver};
use crate::{msg_send, sel};

#[derive(Debug)]
pub(crate) struct CustomObject {
    obj: *mut Object,
}

impl CustomObject {
    fn new(class: &Class) -> Self {
        let ptr: *const Class = class;
        let obj = unsafe { ffi::class_createInstance(ptr.cast(), 0) }.cast();
        CustomObject { obj }
    }
}

// TODO: Remove the need for this hack
impl crate::message::private::Sealed for &CustomObject {}
impl crate::message::private::Sealed for &mut CustomObject {}
impl crate::message::private::Sealed for ManuallyDrop<CustomObject> {}

unsafe impl MessageReceiver for &CustomObject {
    type __Inner = Object;

    #[inline]
    fn __as_raw_receiver(self) -> *mut Object {
        self.obj
    }
}

unsafe impl MessageReceiver for &mut CustomObject {
    type __Inner = Object;

    #[inline]
    fn __as_raw_receiver(self) -> *mut Object {
        self.obj
    }
}

unsafe impl MessageReceiver for ManuallyDrop<CustomObject> {
    type __Inner = Object;

    #[inline]
    fn __as_raw_receiver(self) -> *mut Object {
        self.obj
    }
}

impl Deref for CustomObject {
    type Target = Object;

    fn deref(&self) -> &Object {
        unsafe { self.obj.as_ref().unwrap_unchecked() }
    }
}

impl DerefMut for CustomObject {
    fn deref_mut(&mut self) -> &mut Object {
        unsafe { self.obj.as_mut().unwrap_unchecked() }
    }
}

impl Drop for CustomObject {
    fn drop(&mut self) {
        unsafe {
            #[allow(deprecated)]
            ffi::object_dispose(self.obj.cast());
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub(crate) struct CustomStruct {
    pub(crate) a: u64,
    pub(crate) b: u64,
    pub(crate) c: u64,
    pub(crate) d: u64,
}

unsafe impl Encode for CustomStruct {
    const ENCODING: Encoding = Encoding::Struct(
        "CustomStruct",
        &[u64::ENCODING, u64::ENCODING, u64::ENCODING, u64::ENCODING],
    );
}

pub(crate) fn custom_class() -> &'static Class {
    static REGISTER_CUSTOM_CLASS: Once = Once::new();

    REGISTER_CUSTOM_CLASS.call_once(|| {
        // The runtime will call this method, so it has to be implemented
        extern "C" fn custom_obj_class_initialize(_this: &Class, _cmd: Sel) {}

        let mut builder = ClassBuilder::root(
            "CustomObject",
            custom_obj_class_initialize as extern "C" fn(_, _),
        )
        .unwrap();
        let proto = custom_protocol();

        builder.add_protocol(proto);
        builder.add_ivar::<u32>("_foo");

        unsafe extern "C" fn custom_obj_release(this: *mut Object, _cmd: Sel) {
            // Drop the value
            let _ = CustomObject { obj: this };
        }

        extern "C" fn custom_obj_set_foo(this: &mut Object, _cmd: Sel, foo: u32) {
            unsafe { this.set_ivar::<u32>("_foo", foo) }
        }

        extern "C" fn custom_obj_get_foo(this: &Object, _cmd: Sel) -> u32 {
            unsafe { *this.ivar::<u32>("_foo") }
        }

        extern "C" fn custom_obj_get_foo_reference(this: &Object, _cmd: Sel) -> &u32 {
            unsafe { this.ivar::<u32>("_foo") }
        }

        extern "C" fn custom_obj_get_struct(_this: &Object, _cmd: Sel) -> CustomStruct {
            CustomStruct {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
            }
        }

        extern "C" fn custom_obj_class_method(_this: &Class, _cmd: Sel) -> u32 {
            7
        }

        extern "C" fn custom_obj_set_bar(this: &mut Object, _cmd: Sel, bar: u32) {
            unsafe {
                this.set_ivar::<u32>("_foo", bar);
            }
        }

        extern "C" fn custom_obj_add_number_to_number(
            _this: &Class,
            _cmd: Sel,
            fst: i32,
            snd: i32,
        ) -> i32 {
            fst + snd
        }

        unsafe {
            let release: unsafe extern "C" fn(_, _) = custom_obj_release;
            builder.add_method(sel!(release), release);

            let set_foo: extern "C" fn(_, _, _) = custom_obj_set_foo;
            builder.add_method(sel!(setFoo:), set_foo);
            let get_foo: extern "C" fn(_, _) -> _ = custom_obj_get_foo;
            builder.add_method(sel!(foo), get_foo);
            let get_foo_reference: extern "C" fn(_, _) -> _ = custom_obj_get_foo_reference;
            builder.add_method(sel!(fooReference), get_foo_reference);
            let get_struct: extern "C" fn(_, _) -> CustomStruct = custom_obj_get_struct;
            builder.add_method(sel!(customStruct), get_struct);
            let class_method: extern "C" fn(_, _) -> _ = custom_obj_class_method;
            builder.add_class_method(sel!(classFoo), class_method);

            let protocol_instance_method: extern "C" fn(_, _, _) = custom_obj_set_bar;
            builder.add_method(sel!(setBar:), protocol_instance_method);
            let protocol_class_method: extern "C" fn(_, _, _, _) -> _ =
                custom_obj_add_number_to_number;
            builder.add_class_method(sel!(addNumber:toNumber:), protocol_class_method);
        }

        builder.register();
    });

    // Can't use `class!` here since `CustomObject` is dynamically created.
    Class::get("CustomObject").unwrap()
}

pub(crate) fn custom_protocol() -> &'static Protocol {
    static REGISTER_CUSTOM_PROTOCOL: Once = Once::new();

    REGISTER_CUSTOM_PROTOCOL.call_once(|| {
        let mut builder = ProtocolBuilder::new("CustomProtocol").unwrap();

        builder.add_method_description::<(i32,), ()>(sel!(setBar:), true);
        builder.add_method_description::<(), *const c_char>(sel!(getName), false);
        builder.add_class_method_description::<(i32, i32), i32>(sel!(addNumber:toNumber:), true);

        builder.register();
    });

    Protocol::get("CustomProtocol").unwrap()
}

pub(crate) fn custom_subprotocol() -> &'static Protocol {
    static REGISTER_CUSTOM_SUBPROTOCOL: Once = Once::new();

    REGISTER_CUSTOM_SUBPROTOCOL.call_once(|| {
        let super_proto = custom_protocol();
        let mut builder = ProtocolBuilder::new("CustomSubProtocol").unwrap();

        builder.add_protocol(super_proto);
        builder.add_method_description::<(u32,), u32>(sel!(calculateFoo:), true);

        builder.register();
    });

    Protocol::get("CustomSubProtocol").unwrap()
}

pub(crate) fn custom_object() -> CustomObject {
    CustomObject::new(custom_class())
}

pub(crate) fn custom_subclass() -> &'static Class {
    static REGISTER_CUSTOM_SUBCLASS: Once = Once::new();

    REGISTER_CUSTOM_SUBCLASS.call_once(|| {
        let superclass = custom_class();
        let mut builder = ClassBuilder::new("CustomSubclassObject", superclass).unwrap();

        extern "C" fn custom_subclass_get_foo(this: &Object, _cmd: Sel) -> u32 {
            let foo: u32 = unsafe { msg_send![super(this, custom_class()), foo] };
            foo + 2
        }

        unsafe {
            let get_foo: extern "C" fn(_, _) -> _ = custom_subclass_get_foo;
            builder.add_method(sel!(foo), get_foo);
        }

        builder.register();
    });

    Class::get("CustomSubclassObject").unwrap()
}

pub(crate) fn custom_subclass_object() -> CustomObject {
    CustomObject::new(custom_subclass())
}
