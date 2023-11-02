use std::{env, path::Path};

/// TODO: Better validation of this
///
/// The version is used for providing different behaviour when:
/// - CGException.cpp getObjCPersonality (GNUStep >= 1.7)
/// - Clang.cpp Clang::AddObjCRuntimeArgs (GNUStep >= 2.0)
/// - isLegacyDispatchDefaultForArch (macOS < 10.6, GNUStep < 1.6)
/// - hasNativeARC (macOS < 10.7, iOS < 5)
/// - shouldUseARCFunctionsForRetainRelease (macOS < 10.10, iOS < 8)
/// - shouldUseRuntimeFunctionsForAlloc (macOS < 10.10, iOS < 8)
/// - shouldUseRuntimeFunctionForCombinedAllocInit (macOS >= 10.14.4, iOS >= 12.2, watchOS >= 5.2)
/// - hasOptimizedSetter (macOS >= 10.8, iOS >= 6, GNUStep >= 1.7)
/// - hasSubscripting (macOS < 10.11, iOS < 9)
/// - hasTerminate (macOS < 10.8, iOS < 5)
/// - hasARCUnsafeClaimAutoreleasedReturnValue (macOS >= 10.11, iOS >= 9, watchOS >= 2)
/// - hasEmptyCollections (macOS >= 10.11, iOS >= 9, watchOS >= 2)
/// - ... (incomplete)
///
/// `macosx-fragile` and `gcc` was not considered in this analysis, made on
/// clang version 13's source code:
/// https://github.com/llvm/llvm-project/blob/llvmorg-13.0.0/clang/include/clang/Basic/ObjCRuntime.h
///
/// In short, it's not ultra important, but enables some optimizations if this
/// is specified.
type Version = Option<String>;

// For clang "-fobjc-runtime" support
#[allow(clippy::upper_case_acronyms)]
enum AppleRuntime {
    MacOS(Version),
    IOS(Version),
    TvOS(Version),
    WatchOS(Version),
    // BridgeOS,
}
use AppleRuntime::*;

enum Runtime {
    Apple(AppleRuntime),
    GNUStep(u8, u8),
    WinObjC,
    #[allow(dead_code)]
    ObjFW(Option<String>),
}
use Runtime::*;

fn get_env(env: &str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={}", env);
    match env::var(env) {
        Ok(var) => Some(var),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(var)) => panic!("Invalid unicode for {}: {:?}", env, var),
    }
}

fn main() {
    // The script doesn't depend on our code
    println!("cargo:rerun-if-changed=build.rs");

    let target = env::var("TARGET").unwrap();

    // Used to figure out when BOOL should be i8 vs. bool
    // Matches:
    // aarch64-apple-ios-macabi
    // x86_64-apple-ios-macabi
    if target.ends_with("macabi") {
        println!("cargo:rustc-cfg=target_abi_macabi");
    }

    // Used to set correct image info in `objc2`
    // Matches:
    // aarch64-apple-ios-sim
    // aarch64-apple-watchos-sim
    // x86_64-apple-watchos-sim
    // i386-apple-ios
    // x86_64-apple-ios
    if target.ends_with("sim") || target == "i386-apple-ios" || target == "x86_64-apple-ios" {
        println!("cargo:rustc-cfg=target_simulator");
    }

    // TODO: Figure out when to enable this
    // println!("cargo:rustc-cfg=libobjc2_strict_apple_compat");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let mut apple = env::var_os("CARGO_FEATURE_APPLE").is_some();
    let mut gnustep = env::var_os("CARGO_FEATURE_GNUSTEP_1_7").is_some();
    let objfw = env::var_os("CARGO_FEATURE_UNSTABLE_OBJFW").is_some();

    // Choose defaults when generating docs
    // Only when the crate is being compiled directly
    if cfg!(feature = "unstable-docsrs") {
        if let "macos" | "ios" | "tvos" | "watchos" = &*target_os {
            apple = true;
        } else {
            gnustep = true; // Also winobjc
        }
    }

    let runtime = match (apple, gnustep, objfw) {
        (true, false, false) => {
            Apple(match &*target_os {
                "macos" => MacOS(Some(
                    get_env("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|| "10.7".into()),
                )),
                "ios" => IOS(Some(
                    get_env("IPHONEOS_DEPLOYMENT_TARGET").unwrap_or_else(|| "7.0".into()),
                )),
                "tvos" => TvOS(get_env("TVOS_DEPLOYMENT_TARGET")),
                "watchos" => WatchOS(get_env("WATCHOS_DEPLOYMENT_TARGET")),
                // Choose a sensible default for other platforms that
                // specified `apple`; this is likely not going to work anyhow
                _ => MacOS(None),
            })
        }
        (false, true, false) => {
            // Choose defaults when generating docs
            if cfg!(feature = "unstable-docsrs") {
                if "windows" == target_os {
                    WinObjC
                } else {
                    GNUStep(1, 7)
                }
            } else if env::var_os("CARGO_FEATURE_UNSTABLE_WINOBJC").is_some() {
                WinObjC
            } else if env::var_os("CARGO_FEATURE_GNUSTEP_2_1").is_some() {
                GNUStep(2, 1)
            } else if env::var_os("CARGO_FEATURE_GNUSTEP_2_0").is_some() {
                GNUStep(2, 0)
            } else if env::var_os("CARGO_FEATURE_GNUSTEP_1_9").is_some() {
                GNUStep(1, 9)
            } else if env::var_os("CARGO_FEATURE_GNUSTEP_1_8").is_some() {
                GNUStep(1, 8)
            } else {
                // CARGO_FEATURE_GNUSTEP_1_7
                GNUStep(1, 7)
            }
        }
        (false, false, true) => {
            // For now
            unimplemented!("ObjFW is not yet supported")
            // ObjFW(None)
        }
        (false, false, false) => panic!("Must specify the desired runtime (using cargo features)."),
        _ => panic!("Invalid feature combination; only one runtime may be selected!"),
    };

    // Add `#[cfg(RUNTIME)]` directive
    let runtime_cfg = match runtime {
        Apple(_) => "apple",
        // WinObjC can be treated like GNUStep 1.8
        GNUStep(_, _) | WinObjC => "gnustep",
        ObjFW(_) => "objfw",
    };
    println!("cargo:rustc-cfg={}", runtime_cfg);

    if let Apple(runtime) = &runtime {
        // A few things are defined differently depending on the __OBJC2__
        // variable, which is set for all platforms except 32-bit macOS.
        if let (MacOS(_), "x86") = (runtime, &*target_arch) {
            println!("cargo:rustc-cfg=apple_old");
        } else {
            println!("cargo:rustc-cfg=apple_new");
        }
    }

    let clang_runtime = match &runtime {
        Apple(runtime) => {
            // The fragile runtime is expected on i686-apple-darwin, see:
            // https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/Driver/ToolChains/Darwin.h#L228-L231
            // https://github.com/llvm/llvm-project/blob/release/13.x/clang/lib/Driver/ToolChains/Clang.cpp#L3639-L3640
            let clang_runtime_str = match (runtime, &*target_arch) {
                (MacOS(_), "x86") => "macosx-fragile",
                (MacOS(_), _) => "macosx",
                (IOS(_), _) => "ios",
                (WatchOS(_), _) => "watchos",
                // tvOS doesn't have its own -fobjc-runtime string
                (TvOS(_), _) => "ios",
            };
            match runtime {
                MacOS(version) | IOS(version) | WatchOS(version) | TvOS(version) => {
                    if let Some(version) = version {
                        format!("{}-{}", clang_runtime_str, version)
                    } else {
                        clang_runtime_str.into()
                    }
                }
            }
        }
        // Default in clang is 1.6
        // GNUStep's own default is 1.8
        GNUStep(major, minor) => format!("gnustep-{}.{}", major, minor),
        // WinObjC's libobjc2 is just a fork of gnustep's from version 1.8
        WinObjC => "gnustep-1.8".into(),
        ObjFW(version) => {
            // Default in clang
            let version = version.as_deref().unwrap_or("0.8");
            format!("objfw-{}", version)
        }
    };

    // let gcc_args = match &runtime {
    //     Apple(_) => "-fnext-runtime -fobjc-abi-version=2",
    //     _ => "-fgnu-runtime",
    // };

    // Add CC arguments
    // Assume the compiler is clang; if it isn't, this is probably going to
    // fail anyways, since we're using newer runtimes than GCC supports.
    //
    // TODO: Should add we these, or is it someone else's responsibility?
    // - `-mios-simulator-version-min={}`
    // - `-miphoneos-version-min={}`
    // - `-mmacosx-version-min={}`
    // - ...
    //
    // TODO: -fobjc-weak ?
    let mut cc_args = format!(
        "-fobjc-arc -fobjc-arc-exceptions -fobjc-exceptions -fobjc-runtime={}",
        clang_runtime
    );

    if let Runtime::ObjFW(_) = &runtime {
        // Add compability headers to make `#include <objc/objc.h>` work.
        let compat_headers = Path::new(env!("CARGO_MANIFEST_DIR")).join("compat-headers-objfw");
        cc_args.push_str(" -I");
        cc_args.push_str(compat_headers.to_str().unwrap());
    }

    println!("cargo:cc_args={}", cc_args); // DEP_OBJC_[version]_CC_ARGS

    if let Runtime::ObjFW(_) = &runtime {
        // Link to libobjfw-rt
        println!("cargo:rustc-link-lib=dylib=objfw-rt");
    } else {
        // Link to libobjc
        println!("cargo:rustc-link-lib=dylib=objc");
    }

    // We do this compilation step here instead of in `objc2` to cut down on
    // the total number of build scripts required.
    #[cfg(feature = "unstable-exception")]
    {
        if std::env::var("DOCS_RS").is_ok() {
            // docs.rs doesn't have clang, so skip building this. The
            // documentation will still work since it doesn't need to link.
            //
            // This is independent of the `unstable-docsrs` feature flag; we
            // never want to try invoking clang on docs.rs, whether we're the
            // crate being documented currently, or a dependency of another
            // crate.
            return;
        }
        println!("cargo:rerun-if-changed=extern/exception.m");

        let mut builder = cc::Build::new();
        builder.file("extern/exception.m");

        for flag in cc_args.split(' ') {
            builder.flag(flag);
        }

        builder.compile("librust_objc_sys_0_2_try_catch_exception.a");
    }
}
