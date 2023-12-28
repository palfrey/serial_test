//! # serial_test_derive
//! Helper crate for [serial_test](../serial_test/index.html)

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenTree};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
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
/// Multiple tests with the [serial](macro@serial) attribute are guaranteed to be executed in serial. Ordering
/// of the tests is not guaranteed however. If you have other tests that can be run in parallel, but would clash
/// if run at the same time as the [serial](macro@serial) tests, you can use the [parallel](macro@parallel) attribute.
///
/// If you want different subsets of tests to be serialised with each
/// other, but not depend on other subsets, you can add an argument to [serial](macro@serial), and all calls
/// with identical arguments will be called in serial. e.g.
///
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
///
/// Nested serialised tests (i.e. a [serial](macro@serial) tagged test calling another) are supported
#[proc_macro_attribute]
pub fn serial(attr: TokenStream, input: TokenStream) -> TokenStream {
    local_serial_core(attr.into(), input.into()).into()
}

/// Allows for the creation of parallel Rust tests that won't clash with serial tests
/// ````
/// #[test]
/// #[serial]
/// fn test_serial_one() {
///   // Do things
/// }
///
/// #[test]
/// #[parallel]
/// fn test_parallel_one() {
///   // Do things
/// }
///
/// #[test]
/// #[parallel]
/// fn test_parallel_two() {
///   // Do things
/// }
/// ````
/// Multiple tests with the [parallel](macro@parallel) attribute may run in parallel, but not at the
/// same time as [serial](macro@serial) tests. e.g. in the example code above, `test_parallel_one`
/// and `test_parallel_two` may run at the same time, but `test_serial_one` is guaranteed not to run
/// at the same time as either of them. [parallel](macro@parallel) also takes key arguments for groups
/// of tests as per [serial](macro@serial).
///
/// Note that this has zero effect on [file_serial](macro@file_serial) tests, as that uses a different
/// serialisation mechanism. For that, you want [file_parallel](macro@file_parallel).
#[proc_macro_attribute]
pub fn parallel(attr: TokenStream, input: TokenStream) -> TokenStream {
    local_parallel_core(attr.into(), input.into()).into()
}

/// Allows for the creation of file-serialised Rust tests
/// ````
/// #[test]
/// #[file_serial]
/// fn test_serial_one() {
///   // Do things
/// }
///
/// #[test]
/// #[file_serial]
/// fn test_serial_another() {
///   // Do things
/// }
/// ````
///
/// Multiple tests with the [file_serial](macro@file_serial) attribute are guaranteed to run in serial, as per the [serial](macro@serial)
/// attribute. Note that there are no guarantees about one test with [serial](macro@serial) and another with [file_serial](macro@file_serial)
/// as they lock using different methods, and [file_serial](macro@file_serial) does not support nested serialised tests, but otherwise acts
/// like [serial](macro@serial).
///
/// It also supports an optional `path` arg e.g
/// ````
/// #[test]
/// #[file_serial(key, path => "/tmp/foo")]
/// fn test_serial_one() {
///   // Do things
/// }
///
/// #[test]
/// #[file_serial(key, path => "/tmp/foo")]
/// fn test_serial_another() {
///   // Do things
/// }
/// ````
/// Note that in this case you need to specify the `name` arg as well (as per [serial](macro@serial)). The path defaults to a reasonable temp directory for the OS if not specified.
#[proc_macro_attribute]
#[cfg_attr(docsrs, doc(cfg(feature = "file_locks")))]
pub fn file_serial(attr: TokenStream, input: TokenStream) -> TokenStream {
    fs_serial_core(attr.into(), input.into()).into()
}

/// Allows for the creation of file-serialised parallel Rust tests that won't clash with file-serialised serial tests
/// ````
/// #[test]
/// #[file_serial]
/// fn test_serial_one() {
///   // Do things
/// }
///
/// #[test]
/// #[file_parallel]
/// fn test_parallel_one() {
///   // Do things
/// }
///
/// #[test]
/// #[file_parallel]
/// fn test_parallel_two() {
///   // Do things
/// }
/// ````
/// Effectively, this should behave like [parallel](macro@parallel) but for [file_serial](macro@file_serial).
/// Note that as per [file_serial](macro@file_serial) this doesn't do anything for [serial](macro@serial)/[parallel](macro@parallel) tests.
///
/// It also supports an optional `path` arg e.g
/// ````
/// #[test]
/// #[file_parallel(key, path => "/tmp/foo")]
/// fn test_parallel_one() {
///   // Do things
/// }
///
/// #[test]
/// #[file_parallel(key, path => "/tmp/foo")]
/// fn test_parallel_another() {
///   // Do things
/// }
/// ````
/// Note that in this case you need to specify the `name` arg as well (as per [parallel](macro@parallel)). The path defaults to a reasonable temp directory for the OS if not specified.
#[proc_macro_attribute]
#[cfg_attr(docsrs, doc(cfg(feature = "file_locks")))]
pub fn file_parallel(attr: TokenStream, input: TokenStream) -> TokenStream {
    fs_parallel_core(attr.into(), input.into()).into()
}

// Based off of https://github.com/dtolnay/quote/issues/20#issuecomment-437341743
#[derive(Default, Debug)]
struct QuoteOption<T>(Option<T>);

impl<T: ToTokens> ToTokens for QuoteOption<T> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_all(match self.0 {
            Some(ref t) => quote! { ::std::option::Option::Some(#t) },
            None => quote! { ::std::option::Option::None },
        });
    }
}

#[derive(Default, Debug)]
struct Config {
    names: Vec<String>,
    path: QuoteOption<String>,
}

fn string_from_literal(literal: Literal) -> String {
    let string_literal = literal.to_string();
    if !string_literal.starts_with('\"') || !string_literal.ends_with('\"') {
        panic!("Expected a string literal, got '{}'", string_literal);
    }
    // Hacky way of getting a string without the enclosing quotes
    string_literal[1..string_literal.len() - 1].to_string()
}

fn get_config(attr: proc_macro2::TokenStream) -> Config {
    let mut attrs = attr.into_iter().collect::<Vec<TokenTree>>();
    let mut raw_args: Vec<String> = Vec::new();
    let mut in_path: bool = false;
    let mut path: Option<String> = None;
    while !attrs.is_empty() {
        match attrs.remove(0) {
            TokenTree::Ident(id) if id.to_string().eq_ignore_ascii_case("path") => {
                in_path = true;
            }
            TokenTree::Ident(id) => {
                let name = id.to_string();
                raw_args.push(name);
            }
            x => {
                panic!(
                    "Expected literal as key args (or a 'path => '\"foo\"'), not {}",
                    x
                );
            }
        }
        if in_path {
            if attrs.len() < 3 {
                panic!("Expected a '=> <path>' after 'path'");
            }
            match attrs.remove(0) {
                TokenTree::Punct(p) if p.as_char() == '=' => {}
                x => {
                    panic!("Expected = after path, not {}", x);
                }
            }
            match attrs.remove(0) {
                TokenTree::Punct(p) if p.as_char() == '>' => {}
                x => {
                    panic!("Expected > after path, not {}", x);
                }
            }
            match attrs.remove(0) {
                TokenTree::Literal(literal) => {
                    path = Some(string_from_literal(literal));
                }
                x => {
                    panic!("Expected literals as path arg, not {}", x);
                }
            }
            in_path = false;
        }
        if !attrs.is_empty() {
            match attrs.remove(0) {
                TokenTree::Punct(p) if p.as_char() == ',' => {}
                x => {
                    panic!("Expected , between args, not {}", x);
                }
            }
        }
    }
    if raw_args.is_empty() {
        raw_args.push(String::new());
    }
    raw_args.sort(); // So the keys are always requested in the same order. Avoids dining philosopher issues.
    Config {
        names: raw_args,
        path: QuoteOption(path),
    }
}

fn local_serial_core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let config = get_config(attr);
    serial_setup(input, config, "local")
}

fn local_parallel_core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let config = get_config(attr);
    parallel_setup(input, config, "local")
}

fn fs_serial_core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let config = get_config(attr);
    serial_setup(input, config, "fs")
}

fn fs_parallel_core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let config = get_config(attr);
    parallel_setup(input, config, "fs")
}

fn core_setup(
    input: proc_macro2::TokenStream,
    config: Config,
    prefix: &str,
    kind: &str,
) -> proc_macro2::TokenStream {
    let ast: syn::ItemFn = syn::parse2(input).unwrap();
    let asyncness = ast.sig.asyncness;
    if asyncness.is_some() && cfg!(not(feature = "async")) {
        panic!("async testing attempted with async feature disabled in serial_test!");
    }
    let vis = ast.vis;
    let name = ast.sig.ident;
    let return_type = match ast.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_rarrow, ref box_type) => Some(box_type.deref()),
    };
    let block = ast.block;
    let attrs: Vec<syn::Attribute> = ast.attrs.into_iter().collect();
    let names = config.names;
    let path = config.path;
    if let Some(ret) = return_type {
        match asyncness {
            Some(_) => {
                let fnname = format_ident!("{}_async_{}_core_with_return", prefix, kind);
                let temp_fn = format_ident!("_{}_internal", name);
                quote! {
                    async fn #temp_fn () -> #ret
                        #block

                    #(#attrs)
                    *
                    #vis async fn #name () -> #ret {
                        serial_test::#fnname(vec![#(#names ),*], #path, #temp_fn()).await
                    }
                }
            }
            None => {
                let fnname = format_ident!("{}_{}_core_with_return", prefix, kind);
                quote! {
                    #(#attrs)
                    *
                    #vis fn #name () -> #ret {
                        serial_test::#fnname(vec![#(#names ),*], #path, || #block )
                    }
                }
            }
        }
    } else {
        match asyncness {
            Some(_) => {
                let fnname = format_ident!("{}_async_{}_core", prefix, kind);
                let temp_fn = format_ident!("_{}_internal", name);
                quote! {
                    async fn #temp_fn ()
                        #block

                    #(#attrs)
                    *
                    #vis async fn #name () {
                        serial_test::#fnname(vec![#(#names ),*], #path, #temp_fn()).await;
                    }
                }
            }
            None => {
                let fnname = format_ident!("{}_{}_core", prefix, kind);
                quote! {
                    #(#attrs)
                    *
                    #vis fn #name () {
                        serial_test::#fnname(vec![#(#names ),*], #path, || #block );
                    }
                }
            }
        }
    }
}

fn serial_setup(
    input: proc_macro2::TokenStream,
    config: Config,
    prefix: &str,
) -> proc_macro2::TokenStream {
    core_setup(input, config, prefix, "serial")
}

fn parallel_setup(
    input: proc_macro2::TokenStream,
    config: Config,
    prefix: &str,
) -> proc_macro2::TokenStream {
    core_setup(input, config, prefix, "parallel")
}

#[cfg(test)]
mod tests {
    use super::{fs_serial_core, local_serial_core};
    use quote::quote;
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
                serial_test::local_serial_core(vec![""], ::std::option::Option::None, || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_serial_with_pub() {
        let attrs = proc_macro2::TokenStream::new();
        let input = quote! {
            #[test]
            pub fn foo() {}
        };
        let stream = local_serial_core(attrs.into(), input);
        let compare = quote! {
            #[test]
            pub fn foo () {
                serial_test::local_serial_core(vec![""], ::std::option::Option::None, || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_other_attributes() {
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
            #[ignore]
            #[should_panic(expected = "Testing panic")]
            #[something_else]
            fn foo () {
                serial_test::local_serial_core(vec![""], ::std::option::Option::None,  || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    #[cfg(feature = "async")]
    fn test_serial_async() {
        let attrs = proc_macro2::TokenStream::new();
        let input = quote! {
            async fn foo() {}
        };
        let stream = local_serial_core(attrs.into(), input);
        let compare = quote! {
            async fn _foo_internal () { }
            async fn foo () {
                serial_test::local_async_serial_core(vec![""], ::std::option::Option::None, _foo_internal() ).await;
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    #[cfg(feature = "async")]
    fn test_serial_async_return() {
        let attrs = proc_macro2::TokenStream::new();
        let input = quote! {
            async fn foo() -> Result<(), ()> { Ok(()) }
        };
        let stream = local_serial_core(attrs.into(), input);
        let compare = quote! {
            async fn _foo_internal ()  -> Result<(), ()> { Ok(()) }
            async fn foo () -> Result<(), ()> {
                serial_test::local_async_serial_core_with_return(vec![""], ::std::option::Option::None, _foo_internal() ).await
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_file_serial() {
        let attrs: Vec<_> = quote! { foo }.into_iter().collect();
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
                serial_test::fs_serial_core(vec!["foo"], ::std::option::Option::None, || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_file_serial_no_args() {
        let attrs = proc_macro2::TokenStream::new();
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
                serial_test::fs_serial_core(vec![""], ::std::option::Option::None, || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_file_serial_with_path() {
        let attrs: Vec<_> = quote! { foo, path => "bar_path" }.into_iter().collect();
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
                serial_test::fs_serial_core(vec!["foo"], ::std::option::Option::Some("bar_path"), || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_single_attr() {
        let attrs: Vec<_> = quote! { one}.into_iter().collect();
        let input = quote! {
            #[test]
            fn single() {}
        };
        let stream = local_serial_core(
            proc_macro2::TokenStream::from_iter(attrs.into_iter()),
            input,
        );
        let compare = quote! {
            #[test]
            fn single () {
                serial_test::local_serial_core(vec!["one"], ::std::option::Option::None, || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }

    #[test]
    fn test_multiple_attr() {
        let attrs: Vec<_> = quote! { two, one }.into_iter().collect();
        let input = quote! {
            #[test]
            fn multiple() {}
        };
        let stream = local_serial_core(
            proc_macro2::TokenStream::from_iter(attrs.into_iter()),
            input,
        );
        let compare = quote! {
            #[test]
            fn multiple () {
                serial_test::local_serial_core(vec!["one", "two"], ::std::option::Option::None, || {} );
            }
        };
        assert_eq!(format!("{}", compare), format!("{}", stream));
    }
}
