# serial_test
[![Version](https://img.shields.io/crates/v/serial_test.svg)](https://crates.io/crates/serial_test)
[![Downloads](https://img.shields.io/crates/d/serial_test)](https://crates.io/crates/serial_test)
[![Docs](https://docs.rs/serial_test/badge.svg)](https://docs.rs/serial_test/)
[![MIT license](https://img.shields.io/crates/l/serial_test.svg)](./LICENSE)
[![Build Status](https://github.com/palfrey/serial_test/workflows/Continuous%20integration/badge.svg?branch=main)](https://github.com/palfrey/serial_test/actions)
[![MSRV: 1.51.0](https://flat.badgen.net/badge/MSRV/1.51.0/purple)](https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html)

`serial_test` allows for the creation of serialised Rust tests using the `serial` attribute
e.g.
```rust
#[test]
#[serial]
fn test_serial_one() {
  // Do things
}

#[test]
#[serial]
fn test_serial_another() {
  // Do things
}

#[tokio::test]
#[serial]
async fn test_serial_another() {
  // Do things asynchronously
}
```
Multiple tests with the `serial` attribute are guaranteed to be executed in serial. Ordering of the tests is not guaranteed however. Other tests with the `parallel` attribute may run at the same time as each other, but not at the same time as a test with `serial`. Tests with neither attribute may run at any time and no guarantees are made about their timing!

Note that if you're using an async test reactor attribute (e.g. `tokio::test` or `actix_rt::test`) then they should be listed *before* `serial`, otherwise we
don't get an async function and things break. There's now an error for this case to improve debugging.

For cases like doctests and integration tests where the tests are run as separate processes, we also support `file_serial`, with
similar properties but based off file locking. Note that there are no guarantees about one test with `serial` and another with 
`file_serial` as they lock using different methods, and `parallel` doesn't support `file_serial` yet (patches welcomed!).

## Usage
We require at least Rust 1.51. Upgrades to this will require at least a minor version bump (while in 0.x versions) and a major version bump post-1.0.

Add to your Cargo.toml
```toml
[dev-dependencies]
serial_test = "*"
```

plus `use serial_test::serial;` (for Rust 2018) or
```rust
#[macro_use]
extern crate serial_test;
```
for earlier versions.

You can then either add `#[serial]` or `#[serial(some_text)]` to tests as required.

For each test, a timeout can be specified with the `timeout_ms` parameter to the [serial](macro@serial) attribute. Note that
the timeout is counted from the first invocation of the test, not from the time the previous test was completed. This can
lead to [some unpredictable behavior](https://github.com/palfrey/serial_test/issues/76) based on the number of parallel tests run on the system.
```rust
#[test]
#[serial(timeout_ms = 1000)]
fn test_serial_one() {
  // Do things
}
```