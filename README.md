# serial_test_derive
[![Version](https://img.shields.io/crates/v/serial_test_derive.svg)](https://crates.io/crates/serial_test_derive)
[![Docs](https://docs.rs/serial_test_derive/badge.svg)](https://docs.rs/serial_test_derive/)
![MIT license](https://img.shields.io/crates/l/serial_test_derive.svg)
[![Build Status](https://travis-ci.com/palfrey/serial_test.svg?branch=master)](https://travis-ci.com/palfrey/serial_test)

`serial_test_derive` allows for the creation of serialised Rust tests using the `serial` attribute
e.g.
````
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
````
Multiple tests with the `serial` attribute are guaranteed to be executed in serial. Ordering of the tests is not guaranteed however.

## Usage
We require at least Rust 1.32 for [attribute-like procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html#attribute-macros) support and [proc-macro2](https://github.com/alexcrichton/proc-macro2) being willing to build!

Add to your Cargo.toml
```
[dev-dependencies]
serial_test = "*"
serial_test_derive = "*"
```

plus `use serial_test_derive::serial;` (for Rust 2018) or
```
#[macro_use]
extern crate serial_test_derive;
```
for earlier versions.

You can then either add `#[serial]` or `#[serial(some_text)]` to tests as required.