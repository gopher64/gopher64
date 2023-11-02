use std::env;
use std::path::Path;

fn main() {
    // Only rerun if this file changes; the script doesn't depend on our code
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let mut apple = env::var_os("CARGO_FEATURE_APPLE").is_some();
    let compiler_rt = env::var_os("CARGO_FEATURE_COMPILER_RT").is_some();
    let mut gnustep = env::var_os("CARGO_FEATURE_GNUSTEP_1_7").is_some();
    let objfw = env::var_os("CARGO_FEATURE_UNSTABLE_OBJFW").is_some();

    // Only when the crate is being compiled directly
    if cfg!(feature = "unstable-docsrs") {
        if let "macos" | "ios" | "tvos" | "watchos" = &*target_os {
            apple = true;
            // Add cheaty #[cfg(feature = "apple")] directive
            println!("cargo:rustc-cfg=feature=\"apple\"");
        } else {
            // Also winobjc
            gnustep = true;
            // Add cheaty #[cfg(feature = "gnustep-1-7")] directive
            println!("cargo:rustc-cfg=feature=\"gnustep-1-7\"");
        }
    }

    let mut cc_args = "-fblocks".to_owned();

    match (apple, compiler_rt, gnustep, objfw) {
        (true, false, false, false) => {
            // Link to libclosure (internally called libsystem_blocks), which is
            // exported by libSystem.dylib.
            //
            // Note that System.framework is just a deprecated wrapper over the
            // dynamic library.
            println!("cargo:rustc-link-lib=dylib=System");
            // Alternative: Only link to libsystem_blocks.dylib
            // println!("cargo:rustc-link-search=native=/usr/lib/system");
            // println!("cargo:rustc-link-lib=dylib=system_blocks");
        }
        (false, true, false, false) => {
            println!("cargo:rustc-link-lib=dylib=BlocksRuntime");
        }
        (false, false, true, false) => {
            // Don't link to anything; objc-sys already does that for us!

            // Add GNUStep compability headers to make `#include <Block.h>`
            // work (on newer GNUStep versions these headers are present)
            if env::var_os("CARGO_FEATURE_GNUSTEP_2_0").is_none() {
                let compat_headers =
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("compat-headers/gnustep");
                cc_args.push_str(" -I");
                cc_args.push_str(compat_headers.to_str().unwrap());
            }
        }
        (false, false, false, true) => {
            // Add compability headers to make `#include <Block.h>` work.
            let compat_headers = Path::new(env!("CARGO_MANIFEST_DIR")).join("compat-headers/objfw");
            cc_args.push_str(" -I");
            cc_args.push_str(compat_headers.to_str().unwrap());
            println!("cargo:rustc-link-lib=dylib=objfw");
            unimplemented!("ObjFW is not yet supported")
        }
        // Checked in if-let above
        (false, false, false, false) => {
            panic!("Invalid feature combination; at least one runtime must be selected!")
        }
        (_, _, _, _) => panic!("Invalid feature combination; only one runtime may be selected!"),
    }

    // Add DEP_BLOCK_[version]_CC_ARGS
    println!("cargo:cc_args={}", cc_args);
}
