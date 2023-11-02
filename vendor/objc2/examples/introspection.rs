use objc2::foundation::NSObject;
use objc2::runtime::Class;
use objc2::{sel, ClassType, Encode};

fn main() {
    // Get the class representing `NSObject`
    let cls = NSObject::class();

    // Inspect various properties of the class
    println!("NSObject superclass: {:?}", cls.superclass());
    println!("NSObject size: {}", cls.instance_size());
    println!(
        "-[NSObject alloc] would work: {}",
        cls.responds_to(sel!(alloc))
    );
    println!(
        "+[NSObject alloc] would work: {}",
        cls.metaclass().responds_to(sel!(alloc))
    );

    // Inspect an instance variable on the class
    //
    // Note: You should not rely on the `isa` ivar being available,
    // this is only for demonstration.
    let ivar = cls
        .instance_variable("isa")
        .expect("No ivar with name 'isa' found on NSObject");
    println!(
        "Instance variable {} has type encoding {:?}",
        ivar.name(),
        ivar.type_encoding()
    );
    assert!(<*const Class>::ENCODING.equivalent_to_str(ivar.type_encoding()));

    // Inspect a method of the class
    let method = cls.instance_method(sel!(hash)).unwrap();
    println!(
        "-[NSObject hash] takes {} parameters",
        method.arguments_count()
    );
    #[cfg(feature = "malloc")]
    {
        let hash_return = method.return_type();
        println!("-[NSObject hash] return type: {:?}", hash_return);
        assert!(usize::ENCODING.equivalent_to_str(&hash_return));
    }

    // Create an instance
    let obj = NSObject::new();

    println!("NSObject address: {:p}", obj);

    // Access an ivar of the object
    //
    // As before, you should not rely on the `isa` ivar being available!
    let isa = unsafe { *obj.ivar::<*const Class>("isa") };
    println!("NSObject isa: {:?}", isa);
}
