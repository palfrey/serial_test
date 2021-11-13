//! # serial_test
//! `serial_test` allows for the creation of serialised Rust tests using the [serial](attr.serial.html) attribute
//! e.g.
//! ````
//! #[test]
//! #[serial]
//! fn test_serial_one() {
//!   // Do things
//! }
//!
//! #[test]
//! #[serial]
//! fn test_serial_another() {
//!   // Do things
//! }
//! ````
//! Multiple tests with the [serial](attr.serial.html) attribute are guaranteed to be executed in serial. Ordering
//! of the tests is not guaranteed however.

mod code_lock;
mod file_lock;

pub use code_lock::{
    local_async_serial_core, local_async_serial_core_with_return, local_serial_core,
    local_serial_core_with_return,
};

pub use file_lock::{
    fs_async_serial_core, fs_async_serial_core_with_return, fs_serial_core,
    fs_serial_core_with_return,
};

// Re-export #[serial].
#[allow(unused_imports)]
pub use serial_test_derive::serial;
