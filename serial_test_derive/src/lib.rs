//! # serial_test_derive
//! `serial_test_derive` allows for the creation of serialised Rust tests using the [serial](attr.serial.html) attribute
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
//! of the tests is not guaranteed however

extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree};
use quote::quote;
use syn;

/// Allows for the creation of serialised Rust tests
/// ````
/// #[test]
/// #[serial]
/// fn test_serial_one() {
///   // Do things
/// }
///
/// #[test]
/// #[serial]
/// fn test_serial_another() {
///   // Do things
/// }
/// ````
/// Multiple tests with the [serial](attr.serial.html) attribute are guaranteed to be executed in serial. Ordering
/// of the tests is not guaranteed however. If you want different subsets of tests to be serialised with each
/// other, but not depend on other subsets, you can add an argument to [serial](attr.serial.html), and all calls
/// with identical arguments will be called in serial. e.g.
/// ````
/// #[test]
/// #[serial(something)]
/// fn test_serial_one() {
///   // Do things
/// }
///
/// #[test]
/// #[serial(something)]
/// fn test_serial_another() {
///   // Do things
/// }
///
/// #[test]
/// #[serial(other)]
/// fn test_serial_third() {
///   // Do things
/// }
///
/// #[test]
/// #[serial(other)]
/// fn test_serial_fourth() {
///   // Do things
/// }
/// ````
/// `test_serial_one` and `test_serial_another` will be executed in serial, as will `test_serial_third` and `test_serial_fourth`
/// but neither sequence will be blocked by the other
#[proc_macro_attribute]
pub fn serial(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = attr.into_iter().collect::<Vec<TokenTree>>();
    let key = match attrs.len() {
        0 => "".to_string(),
        1 => {
            if let TokenTree::Ident(id) = &attrs[0] {
                id.to_string()
            } else {
                panic!("Expected a single name as argument");
            }
        }
        _ => {
            panic!("Expected either 0 or 1 arguments");
        }
    };
    let ast: syn::ItemFn = syn::parse(input).unwrap();
    let name = ast.ident;
    let block = ast.block;
    let gen = quote! {
        fn #name () {
            serial_test::serial_core(#key, || {
                #block
            });
        }
    };
    return gen.into();
}
