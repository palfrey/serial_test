//! # serial_test_derive
//! Helper crate for [serial_test](../serial_test/index.html)

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::{format_ident, quote};
use std::ops::Deref;

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
#[proc_macro_error]
pub fn serial(attr: TokenStream, input: TokenStream) -> TokenStream {
    local_serial_core(attr.into(), input.into()).into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn file_serial(attr: TokenStream, input: TokenStream) -> TokenStream {
    fs_serial_core(attr.into(), input.into()).into()
}

fn local_serial_core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let attrs = attr.into_iter().collect::<Vec<TokenTree>>();
    let key = match attrs.len() {
        0 => "".to_string(),
        1 => {
            if let TokenTree::Ident(id) = &attrs[0] {
                id.to_string()
            } else {
                panic!("Expected a single name as argument, got {:?}", attrs);
            }
        }
        n => {
            panic!("Expected either 0 or 1 arguments, got {}: {:?}", n, attrs);
        }
    };
    serial_setup(input, &vec![key], "local")
}

fn fs_serial_core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let attrs = attr.into_iter().collect::<Vec<TokenTree>>();
    match attrs.len() {
        0 => serial_setup(input, &vec!["".to_string(), "".to_string()], "fs"),
        1 => {
            if let TokenTree::Ident(id) = &attrs[0] {
                serial_setup(input, &vec![id.to_string(), "".to_string()], "fs")
            } else {
                panic!("Expected a single name as argument, got {:?}", attrs);
            }
        }
        2 => {
            let key;
            let path;
            if let TokenTree::Ident(id) = &attrs[0] {
                key = id.to_string()
            } else {
                panic!("Expected name as first argument, got {:?}", attrs);
            }
            if let TokenTree::Ident(id) = &attrs[1] {
                path = id.to_string()
            } else {
                panic!("Expected path as second argument, got {:?}", attrs);
            }
            serial_setup(input, &vec![key, path], "fs")
        }
        n => {
            panic!("Expected either 0 or 1 arguments, got {}: {:?}", n, attrs);
        }
    }
}

fn serial_setup<'a>(
    input: proc_macro2::TokenStream,
    args: &[String],
    prefix: &str,
) -> proc_macro2::TokenStream {
    let ast: syn::ItemFn = syn::parse2(input).unwrap();
    let asyncness = ast.sig.asyncness;
    let name = ast.sig.ident;
    let return_type = match ast.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_rarrow, ref box_type) => Some(box_type.deref()),
    };
    let block = ast.block;
    let attrs: Vec<syn::Attribute> = ast
        .attrs
        .into_iter()
        .filter(|at| {
            if let Ok(m) = at.parse_meta() {
                let path = m.path();
                if asyncness.is_some()
                    && path.segments.len() == 2
                    && path.segments[1].ident == "test"
                {
                    // We assume that any 2-part attribute with the second part as "test" on an async function
                    // is the "do this test with reactor" wrapper. This is true for actix, tokio and async_std.
                    abort_call_site!("Found async test attribute after serial, which will break");
                }

                // we skip ignore/should_panic because the test framework already deals with it
                !(path.is_ident("ignore") || path.is_ident("should_panic"))
            } else {
                true
            }
        })
        .collect();
    if let Some(ret) = return_type {
        match asyncness {
            Some(_) => {
                let fnname = format_ident!("{}_async_serial_core_with_return", prefix);
                quote! {
                    #(#attrs)
                    *
                    async fn #name () -> #ret {
                        serial_test::#fnname(#(#args ),*, || async #block ).await;
                    }
                }
            }
            None => {
                let fnname = format_ident!("{}_serial_core_with_return", prefix);
                quote! {
                    #(#attrs)
                    *
                    fn #name () -> #ret {
                        serial_test::#fnname(#(#args ),*, || #block )
                    }
                }
            }
        }
    } else {
        match asyncness {
            Some(_) => {
                let fnname = format_ident!("{}_async_serial_core", prefix);
                quote! {
                    #(#attrs)
                    *
                    async fn #name () {
                        serial_test::#fnname(#(#args ),*, || async #block ).await;
                    }
                }
            }
            None => {
                let fnname = format_ident!("{}_serial_core", prefix);
                quote! {
                    #(#attrs)
                    *
                    fn #name () {
                        serial_test::#fnname(#(#args ),*, || #block );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{format_ident, fs_serial_core, local_serial_core, quote, TokenTree};
    use std::iter::FromIterator;

    #[test]
    fn test_serial() {
        let attrs = proc_macro2::TokenStream::new();
        let input = quote! {
            #[test]
            fn foo() {}
        };
        let stream = local_serial_core(attrs.into(), input);
        let compare = quote! {
            #[test]
            fn foo () {
                serial_test::local_serial_core("", || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_stripped_attributes() {
        let _ = env_logger::builder().is_test(true).try_init();
        let attrs = proc_macro2::TokenStream::new();
        let input = quote! {
            #[test]
            #[ignore]
            #[should_panic(expected = "Testing panic")]
            #[something_else]
            fn foo() {}
        };
        let stream = local_serial_core(attrs.into(), input);
        let compare = quote! {
            #[test]
            #[something_else]
            fn foo () {
                serial_test::local_serial_core("", || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_serial_async() {
        let attrs = proc_macro2::TokenStream::new();
        let input = quote! {
            async fn foo() {}
        };
        let stream = local_serial_core(attrs.into(), input);
        let compare = quote! {
            async fn foo () {
                serial_test::local_async_serial_core("", || async {} ).await;
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_serial_async_return() {
        let attrs = proc_macro2::TokenStream::new();
        let input = quote! {
            async fn foo() -> Result<(), ()> { Ok(()) }
        };
        let stream = local_serial_core(attrs.into(), input);
        let compare = quote! {
            async fn foo () -> Result<(), ()> {
                serial_test::local_async_serial_core_with_return("", || async { Ok(()) } ).await;
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    // 1.54 needed for https://github.com/rust-lang/rust/commit/9daf546b77dbeab7754a80d7336cd8d00c6746e4 change in note message
    #[rustversion::since(1.54)]
    #[test]
    fn test_serial_async_before_wrapper() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/broken/test_serial_async_before_wrapper.rs");
    }

    #[test]
    fn test_file_serial() {
        let attrs = vec![TokenTree::Ident(format_ident!("foo"))];
        let input = quote! {
            #[test]
            fn foo() {}
        };
        let stream = fs_serial_core(
            proc_macro2::TokenStream::from_iter(attrs.into_iter()),
            input,
        );
        let compare = quote! {
            #[test]
            fn foo () {
                serial_test::fs_serial_core("foo", "", || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_file_serial_with_path() {
        let attrs = vec![
            TokenTree::Ident(format_ident!("foo")),
            TokenTree::Ident(format_ident!("bar_path")),
        ];
        let input = quote! {
            #[test]
            fn foo() {}
        };
        let stream = fs_serial_core(
            proc_macro2::TokenStream::from_iter(attrs.into_iter()),
            input,
        );
        let compare = quote! {
            #[test]
            fn foo () {
                serial_test::fs_serial_core("foo", "bar_path", || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }
}
