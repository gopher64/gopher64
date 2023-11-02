#![cfg_attr(not(all(feature = "apple", target_os = "macos")), allow(unused))]
use objc2::declare::{Ivar, IvarDrop};
use objc2::foundation::{NSCopying, NSObject, NSString};
use objc2::rc::{Id, Shared};
use objc2::runtime::Object;
use objc2::{declare_class, extern_class, msg_send, msg_send_id, ns_string, ClassType};

#[cfg(all(feature = "apple", target_os = "macos"))]
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[cfg(all(feature = "apple", target_os = "macos"))]
extern_class!(
    #[derive(Debug)]
    struct NSResponder;

    unsafe impl ClassType for NSResponder {
        type Super = NSObject;
    }
);

#[cfg(all(feature = "apple", target_os = "macos"))]
declare_class!(
    #[derive(Debug)]
    struct CustomAppDelegate {
        pub ivar: u8,
        another_ivar: bool,
        box_ivar: IvarDrop<Box<i32>>,
        maybe_box_ivar: IvarDrop<Option<Box<i32>>>,
        id_ivar: IvarDrop<Id<NSString, Shared>>,
        maybe_id_ivar: IvarDrop<Option<Id<NSString, Shared>>>,
    }

    unsafe impl ClassType for CustomAppDelegate {
        #[inherits(NSObject)]
        type Super = NSResponder;
        const NAME: &'static str = "MyCustomAppDelegate";
    }

    unsafe impl CustomAppDelegate {
        #[sel(initWith:another:)]
        fn init_with(self: &mut Self, ivar: u8, another_ivar: bool) -> Option<&mut Self> {
            let this: Option<&mut Self> = unsafe { msg_send![super(self), init] };

            // TODO: `ns_string` can't be used inside closures; investigate!
            let s = ns_string!("def");

            this.map(|this| {
                Ivar::write(&mut this.ivar, ivar);
                *this.another_ivar = another_ivar;
                *this.maybe_box_ivar = None;
                *this.maybe_id_ivar = Some(s.copy());
                Ivar::write(&mut this.box_ivar, Box::new(2));
                Ivar::write(&mut this.id_ivar, NSString::from_str("abc"));
                this
            })
        }

        #[sel(myClassMethod)]
        fn my_class_method() {
            println!("A class method!");
        }
    }

    // For some reason, `NSApplicationDelegate` is not a "real" protocol we
    // can retrieve using `objc_getProtocol` - it seems it is created by
    // `clang` only when used in Objective-C...
    //
    // TODO: Investigate this!
    unsafe impl CustomAppDelegate {
        /// This is `unsafe` because it expects `sender` to be valid
        #[sel(applicationDidFinishLaunching:)]
        unsafe fn did_finish_launching(&self, sender: *mut Object) {
            println!("Did finish launching!");
            // Do something with `sender`
            dbg!(sender);
        }

        /// Some comment before `sel`.
        #[sel(applicationWillTerminate:)]
        /// Some comment after `sel`.
        fn will_terminate(&self, _: *mut Object) {
            println!("Will terminate!");
        }
    }
);

#[cfg(all(feature = "apple", target_os = "macos"))]
impl CustomAppDelegate {
    pub fn new(ivar: u8, another_ivar: bool) -> Id<Self, Shared> {
        let cls = Self::class();
        unsafe {
            msg_send_id![
                msg_send_id![cls, alloc],
                initWith: ivar,
                another: another_ivar,
            ]
        }
    }
}

#[cfg(all(feature = "apple", target_os = "macos"))]
fn main() {
    let delegate = CustomAppDelegate::new(42, true);

    println!("{:?}", delegate);
    println!("{:?}", delegate.ivar);
    println!("{:?}", delegate.another_ivar);
    println!("{:?}", delegate.box_ivar);
    println!("{:?}", delegate.maybe_box_ivar);
    println!("{:?}", delegate.id_ivar);
    println!("{:?}", delegate.maybe_id_ivar);
}

#[cfg(not(all(feature = "apple", target_os = "macos")))]
fn main() {
    panic!("This example uses AppKit, which is only present on macOS");
}
