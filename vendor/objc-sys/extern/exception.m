// Don't include any headers, cross compilation is difficult to set up
// properly in such situations.

/// We're linking to `libobjc` in build.rs, so this should be available.
///
/// See <https://clang.llvm.org/docs/AutomaticReferenceCounting.html#arc-runtime-objc-retain>.
id objc_retain(id value);

/// Unsure how C name resolution works, so we make sure to version this symbol.
///
/// Return `unsigned char` since it is guaranteed to be `u8` on all platforms.
unsigned char rust_objc_sys_0_2_try_catch_exception(void (*f)(void *), void *context, id *error) {
    @try {
        f(context);
        if (error) {
            *error = (id)0; // nil
        }
        return 0;
    } @catch (id exception) {
        if (error) {
            *error = objc_retain(exception);
        }
        return 1;
    }
}
