//! # Apple's C language extension of blocks
//!
//! C Blocks are effectively the C-equivalent of Rust's closures, in that they
//! have the ability to capture their environments.
//!
//! This crate provides capabilities to create and invoke these blocks, in an
//! ergonomic "Rust-centric" fashion.
//!
//! For more information on the specifics of the block implementation, see the
//! [C language specification][lang] and the [ABI specification][ABI].
//!
//! (Note that while this library can be used separately from Objective-C,
//! they're most commonly used together).
//!
//! ## Invoking blocks
//!
//! The [`Block`] struct is used for invoking blocks from Objective-C. For
//! example, consider this Objective-C function that takes a block as a
//! parameter, executes the block with some arguments, and returns the result:
//!
//! ```objc
//! #include <stdint.h>
//! #include <Block.h>
//! int32_t run_block(int32_t (^block)(int32_t, int32_t)) {
//!     return block(5, 8);
//! }
//! ```
//!
//! We could write the equivalent function in Rust like this:
//!
//! ```
//! use block2::Block;
//! unsafe fn run_block(block: &Block<(i32, i32), i32>) -> i32 {
//!     block.call((5, 8))
//! }
//! ```
//!
//! Note the extra parentheses in the `call` method, since the arguments must
//! be passed as a tuple.
//!
//! ## Creating blocks
//!
//! Creating a block to pass to Objective-C can be done with the
//! [`ConcreteBlock`] struct. For example, to create a block that adds two
//! integers, we could write:
//!
//! ```
//! use block2::ConcreteBlock;
//! let block = ConcreteBlock::new(|a: i32, b: i32| a + b);
//! let block = block.copy();
//! assert_eq!(unsafe { block.call((5, 8)) }, 13);
//! ```
//!
//! It is important to copy your block to the heap (with the [`copy`] method)
//! before passing it to Objective-C; this is because our [`ConcreteBlock`] is
//! only meant to be copied once, and we can enforce this in Rust, but if
//! Objective-C code were to copy it twice we could have a double free.
//!
//! [`copy`]: ConcreteBlock::copy
//!
//! As an optimization if your block doesn't capture any variables, you can
//! use the [`global_block!`] macro to create a static block:
//!
//! ```
//! use block2::global_block;
//! global_block! {
//!     static MY_BLOCK = || -> f32 {
//!         10.0
//!     };
//! }
//! assert_eq!(unsafe { MY_BLOCK.call(()) }, 10.0);
//! ```
//!
//! [lang]: https://clang.llvm.org/docs/BlockLanguageSpec.html
//! [ABI]: http://clang.llvm.org/docs/Block-ABI-Apple.html

#![no_std]
#![warn(elided_lifetimes_in_paths)]
#![warn(missing_docs)]
#![deny(non_ascii_idents)]
#![warn(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::cargo)]
#![warn(clippy::ptr_as_ptr)]
// Update in Cargo.toml as well.
#![doc(html_root_url = "https://docs.rs/block2/0.2.0-alpha.6")]

extern crate alloc;
extern crate std;

#[cfg(not(feature = "std"))]
compile_error!("The `std` feature currently must be enabled.");

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
extern "C" {}

pub use block_sys as ffi;

mod block;
mod concrete_block;
mod debug;
mod global;
mod rc_block;

pub use block::{Block, BlockArguments};
pub use concrete_block::{ConcreteBlock, IntoConcreteBlock};
pub use global::GlobalBlock;
pub use rc_block::RcBlock;
