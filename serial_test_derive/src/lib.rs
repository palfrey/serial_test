//! # serial_test_derive
//! Helper crate for [serial_test](../serial_test/index.html)

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::quote;
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
    serial_core(attr.into(), input.into()).into()
}

fn serial_core(
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
            Some(_) => quote! {
                #(#attrs)
                *
                async fn #name () -> #ret {
                    serial_test::async_serial_core_with_return(#key, || async #block ).await;
                }
            },
            None => quote! {
                #(#attrs)
                *
                fn #name () -> #ret {
                    serial_test::serial_core_with_return(#key, || #block )
                }
            },
        }
    } else {
        match asyncness {
            Some(_) => quote! {
                #(#attrs)
                *
                async fn #name () {
                    serial_test::async_serial_core(#key, || async #block ).await;
                }
            },
            None => quote! {
                #(#attrs)
                *
                fn #name () {
                    serial_test::serial_core(#key, || #block );
                }
            },
        }
    }
}

#[test]
fn test_serial() {
    let attrs = proc_macro2::TokenStream::new();
    let input = quote! {
        #[test]
        fn foo() {}
    };
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[test]
        fn foo () {
            serial_test::serial_core("", || {} );
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
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[test]
        #[something_else]
        fn foo () {
            serial_test::serial_core("", || {} );
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
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        async fn foo () {
            serial_test::async_serial_core("", || async {} ).await;
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
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        async fn foo () -> Result<(), ()> {
            serial_test::async_serial_core_with_return("", || async { Ok(()) } ).await;
        }
    };
    assert_eq!(format!("{}", compare), format!("{}", stream));
}

#[test]
#[should_panic = "proc-macro-error API cannot be used outside of"]
fn test_serial_async_before_wrapper() {
    let attrs = proc_macro2::TokenStream::new();
    let input = quote! {
        #[serial]
        #[actix_rt::test]
        async fn test_async_serial_no_arg_actix() {}
    };

    // This will panic because we're trying to call into proc-macro-error outside of a proc macro
    // Kinda a side-effect of proc macros being hard to test TBH, and so we can't actually check for the proper error message
    serial_core(attrs.into(), input.into());
}
