#![warn(missing_docs)]
#![allow(clippy::upper_case_acronyms)]
// TODO: https://github.com/jni-rs/jni-rs/issues/348
#![allow(clippy::not_unsafe_ptr_arg_deref)]

//! # Safe JNI Bindings in Rust
//!
//! This crate provides a (mostly) safe way to implement methods in Java using
//! the JNI. Because who wants to *actually* write Java?
//!
//! ## Getting Started
//!
//! Naturally, any ffi-related project is going to require some code in both
//! languages that we're trying to make communicate. Java requires all native
//! methods to adhere to the Java Native Interface (JNI), so we first have to
//! define our function signature from Java, and then we can write Rust that
//! will adhere to it.
//!
//! ### The Java side
//!
//! First, you need a Java class definition. `HelloWorld.java`:
//!
//! ```java
//! class HelloWorld {
//!     // This declares that the static `hello` method will be provided
//!     // a native library.
//!     private static native String hello(String input);
//!
//!     static {
//!         // This actually loads the shared object that we'll be creating.
//!         // The actual location of the .so or .dll may differ based on your
//!         // platform.
//!         System.loadLibrary("mylib");
//!     }
//!
//!     // The rest is just regular ol' Java!
//!     public static void main(String[] args) {
//!         String output = HelloWorld.hello("josh");
//!         System.out.println(output);
//!     }
//! }
//! ```
//!
//! Compile this to a class file with `javac HelloWorld.java`.
//!
//! Trying to run it now will give us the error `Exception in thread "main"
//! java.lang.UnsatisfiedLinkError: no mylib in java.library.path` since we
//! haven't written our native code yet.
//!
//! To do that, first we need the name and type signature that our Rust function
//! needs to adhere to. Luckily, the Java compiler can generate that for you!
//! Run `javac -h . HelloWorld.java` and you'll get a `HelloWorld.h` output to your
//! directory. It should look something like this:
//!
//! ```c
//! /* DO NOT EDIT THIS FILE - it is machine generated */
//! #include <jni.h>
//! /* Header for class HelloWorld */
//!
//! #ifndef _Included_HelloWorld
//! #define _Included_HelloWorld
//! #ifdef __cplusplus
//! extern "C" {
//! #endif
//! /*
//!  * Class:     HelloWorld
//!  * Method:    hello
//!  * Signature: (Ljava/lang/String;)Ljava/lang/String;
//!  */
//! JNIEXPORT jstring JNICALL Java_HelloWorld_hello
//!   (JNIEnv *, jclass, jstring);
//!
//! #ifdef __cplusplus
//! }
//! #endif
//! #endif
//! ```
//!
//! It's a C header, but luckily for us, the types will mostly match up. Let's
//! make our crate that's going to compile to our native library.
//!
//! ### The Rust side
//!
//! Create your crate with `cargo new mylib`. This will create a directory
//! `mylib` that has everything needed to build an basic crate with `cargo`. We
//! need to make a couple of changes to `Cargo.toml` before we do anything else.
//!
//! * Under `[dependencies]`, add `jni = "0.21.1"`
//! * Add a new `[lib]` section and under it, `crate_type = ["cdylib"]`.
//!
//! Now, if you run `cargo build` from inside the crate directory, you should
//! see a `libmylib.so` (if you're on linux) or a `libmylib.dylib` (if you are on OSX) in the `target/debug`
//! directory.
//!
//! The last thing we need to do is to define our exported method. Add this to
//! your crate's `src/lib.rs`:
//!
//! ```rust,no_run
//! // This is the interface to the JVM that we'll call the majority of our
//! // methods on.
//! use jni::JNIEnv;
//!
//! // These objects are what you should use as arguments to your native
//! // function. They carry extra lifetime information to prevent them escaping
//! // this context and getting used after being GC'd.
//! use jni::objects::{JClass, JString};
//!
//! // This is just a pointer. We'll be returning it from our function. We
//! // can't return one of the objects with lifetime information because the
//! // lifetime checker won't let us.
//! use jni::sys::jstring;
//!
//! // This keeps Rust from "mangling" the name and making it unique for this
//! // crate.
//! #[no_mangle]
//! pub extern "system" fn Java_HelloWorld_hello<'local>(mut env: JNIEnv<'local>,
//! // This is the class that owns our static method. It's not going to be used,
//! // but still must be present to match the expected signature of a static
//! // native method.
//!                                                      class: JClass<'local>,
//!                                                      input: JString<'local>)
//!                                                      -> jstring {
//!     // First, we have to get the string out of Java. Check out the `strings`
//!     // module for more info on how this works.
//!     let input: String =
//!         env.get_string(&input).expect("Couldn't get java string!").into();
//!
//!     // Then we have to create a new Java string to return. Again, more info
//!     // in the `strings` module.
//!     let output = env.new_string(format!("Hello, {}!", input))
//!         .expect("Couldn't create java string!");
//!
//!     // Finally, extract the raw pointer to return.
//!     output.into_raw()
//! }
//! ```
//!
//! Note that the type signature for our function is almost identical to the one
//! from the generated header, aside from our lifetime-carrying arguments.
//!
//! ### Final steps
//!
//! That's it! Build your crate and try to run your Java class again.
//!
//! ... Same error as before you say? Well that's because JVM is looking for
//! `mylib` in all the wrong places. This will differ by platform thanks to
//! different linker/loader semantics, but on Linux, you can simply `export
//! LD_LIBRARY_PATH=/path/to/mylib/target/debug`. Now, you should get the
//! expected output `Hello, josh!` from your Java class.
//!
//! ## Launching JVM from Rust
//!
//! It is possible to launch a JVM from a native process using the [Invocation API], provided
//! by [`JavaVM`](struct.JavaVM.html).
//!
//! ## See Also
//!
//! ### Examples
//! - [Example project][jni-rs-example]
//! - Our [integration tests][jni-rs-its] and [benchmarks][jni-rs-benches]
//!
//! ### JNI Documentation
//! - [Java Native Interface Specification][jni-spec]
//! - [JNI tips][jni-tips] — general tips on JNI development and some Android-specific
//!
//! ### Open-Source Users
//! - The Servo browser engine Android [port][users-servo]
//! - The Exonum framework [Java Binding][users-ejb]
//! - MaidSafe [Java Binding][users-maidsafe]
//!
//! ### Other Projects Simplifying Java and Rust Communication
//! - Consider [JNR][projects-jnr] if you just need to use a native library with C interface
//! - Watch OpenJDK [Project Panama][projects-panama] which aims to enable using native libraries
//!   with no JNI code
//! - Consider [GraalVM][projects-graalvm] — a recently released VM that gives zero-cost
//!   interoperability between various languages (including Java and [Rust][graalvm-rust] compiled
//!   into LLVM-bitcode)
//!
//! [Invocation API]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/invocation.html
//! [jni-spec]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/index.html
//! [jni-tips]: https://developer.android.com/training/articles/perf-jni
//! [jni-rs-example]: https://github.com/jni-rs/jni-rs/tree/master/example
//! [jni-rs-its]: https://github.com/jni-rs/jni-rs/tree/master/tests
//! [jni-rs-benches]: https://github.com/jni-rs/jni-rs/tree/master/benches
//! [users-servo]: https://github.com/servo/servo/tree/master/ports/libsimpleservo
//! [users-ejb]: https://github.com/exonum/exonum-java-binding/tree/master/exonum-java-binding/core/rust
//! [users-maidsafe]: https://github.com/maidsafe/safe_client_libs/tree/master/safe_app_jni
//! [projects-jnr]: https://github.com/jnr/jnr-ffi/
//! [projects-graalvm]: http://www.graalvm.org/docs/why-graal/#for-java-programs
//! [graalvm-rust]: http://www.graalvm.org/docs/reference-manual/languages/llvm/#running-rust
//! [projects-panama]: https://jdk.java.net/panama/

/// `jni-sys` re-exports
pub mod sys;

mod wrapper {
    mod version;
    pub use self::version::*;

    #[macro_use]
    mod macros;

    /// Errors. Do you really need more explanation?
    pub mod errors;

    /// Descriptors for classes and method IDs.
    pub mod descriptors;

    /// Parser for java type signatures.
    pub mod signature;

    /// Wrappers for object pointers returned from the JVM.
    pub mod objects;

    /// String types for going to/from java strings.
    pub mod strings;

    /// Actual communication with the JVM.
    mod jnienv;
    pub use self::jnienv::*;

    /// Java VM interface.
    mod java_vm;
    pub use self::java_vm::*;

    /// Optional thread attachment manager.
    mod executor;
    pub use self::executor::*;
}

pub use wrapper::*;
