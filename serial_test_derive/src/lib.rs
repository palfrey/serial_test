//! # serial_test_derive
//! Helper crate for [serial_test](../serial_test/index.html)

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{TokenTree, Group, Ident};
use quote::{quote, ToTokens};
use std::ops::Deref;
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
    return serial_core(attr.into(), input.into()).into();
}

fn serial_core(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let attrs = attr.into_iter().collect::<Vec<TokenTree>>();
    let (key,reactor) = match attrs.len() {
        0 => ("".to_string(), None),
        1 => {
            match &attrs[0]  {
                TokenTree::Ident(id) => (id.to_string(), None),
                TokenTree::Group(g) => ("".into(), Some(parse_reactor(&attrs, g))),
                _ => panic!("Expected a single name or {{reactor_name}} as argument, got {:?}", attrs),
            }
        }
        2 => {
            (match &attrs[0]  {
                TokenTree::Ident(id) => id.to_string(),
                _ => panic!("Expected a single name as first argument, got {:?}", attrs),
            },
            match &attrs[1]  {
                TokenTree::Group(g) => Some(parse_reactor(&attrs, g)),
                _ => panic!("Expected a {{reactor_name}} as second argument, got {:?}", attrs),
            })
        }
        n => {
            panic!("Expected either 0, 1 or 2 arguments, got {}: {:?}", n, attrs);
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
    let mut gen_reactor = true;
    let mut gen_test_tag = true;
    let attrs = ast.attrs;
    for at in &attrs {
        if let Ok(m) = at.parse_meta() {
            let path = m.path();
            if path.is_ident("test") {
                gen_test_tag = false;
            }

            if path.segments.len() == 2 {
                if path.segments[1].ident.to_string() == "test"
                    && (path.segments[0].ident.to_string() == "tokio"
                        || path.segments[0].ident.to_string() == "actix_rt")
                {
                    // we will generate reactor code ourselves for async fns
                    gen_reactor = false;
                    println!(
                        "will not generate reactor code since test is wrapped with: {:?} [{}]",
                        path.to_token_stream(),
                        path.segments.len()
                    );
                }
            }
        };
    }
    let test_tag = match gen_test_tag {
        false => quote!{},
        true => quote!{ #[test] },
    };

    let reactor = reactor
        .map(|_r| quote!{tokio::runtime::Runtime::new().unwrap().block_on})
        .unwrap_or(quote!{actix_rt::System::new("test").block_on});

    let gen = if let Some(ret) = return_type {
        match asyncness {
            Some(_) => {
                if gen_reactor {
                    quote! {
                        #test_tag
                        #(#attrs)*
                        fn #name () -> #ret {
                            #reactor (
                                serial_test::async_serial_core_with_return(#key, async { #block } )
                            )
                        }
                    }
                } else {
                    quote! {
                        #(#attrs)*
                        async fn #name () -> #ret {
                            serial_test::async_serial_core_with_return(#key, || async { #block }).await;
                        }
                    }
                }
            }
            None => quote! {
                #test_tag
                #(#attrs)*
                fn #name () -> #ret {
                    serial_test::serial_core_with_return(#key, || {
                        #block
                    })
                }
            },
        }
    } else {
        match asyncness {
            Some(_) => {
                if gen_reactor {
                    quote! {
                        #test_tag
                        #(#attrs)*
                        fn #name () {
                            #reactor (
                                serial_test::async_serial_core(#key, async { #block } )
                            )
                        }
                    }
                } else {
                    quote! {
                        #(#attrs)*
                        async fn #name () {
                            serial_test::async_serial_core(#key, || async { #block }).await;
                        }
                    }
                }
            }
            None => quote! {
                #test_tag
                #(#attrs)*
                fn #name () {
                    serial_test::serial_core(#key, || {
                        #block
                    });
                }
            },
        }
    };
    return gen.into();
}

fn parse_reactor(attrs: &Vec<TokenTree>, g: &Group) -> Ident {
    let g :Vec<TokenTree>= g.stream().into_iter().collect();
    if g.len() != 1 {
        panic!("Expected a single {{reactor_name}} as argument, got {:?}", attrs);
    }
    match &g[0] {
        TokenTree::Ident(reactor) => reactor.clone(),
        _ => panic!("Expected a single {{reactor_name}} as argument, got {:?}", attrs),
    }
}

#[test]
fn test_serial() {
    let attrs = proc_macro2::TokenStream::new();
    let input = quote! {
        fn foo() {}
    };
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[test]
        fn foo () {
            serial_test::serial_core("", || {
                {}
            });
        }
    };
    assert_eq!(format!("{}", compare), format!("{}", stream));
}

#[test]
fn test_stripped_attributes() {
    let _ = env_logger::builder().is_test(true).try_init();
    let attrs = proc_macro2::TokenStream::new();
    let input = quote! {
        #[ignore]
        #[should_panic(expected = "Testing panic")]
        #[something_else]
        fn foo() {}
    };
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[test]
        #[ignore]
        #[should_panic(expected = "Testing panic")]
        #[something_else]
        fn foo () {
            serial_test::serial_core("", || {
                {}
            });
        }
    };
    assert_eq!(format!("{}", compare), format!("{}", stream));
}

#[test]
fn test_serial_async() {
    let attrs = proc_macro2::TokenStream::new();
    let input = quote! {
        #[actix_rt::test]
        async fn foo() {}
    };
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[actix_rt::test]
        async fn foo () {
            serial_test::async_serial_core("", || async {
                {}
            }).await;
        }
    };
    assert_eq!(format!("{}", compare), format!("{}", stream));
}

#[test]
fn test_serial_async_return() {
    let attrs = proc_macro2::TokenStream::new();
    let input = quote! {
        #[tokio::test]
        async fn foo() -> Result<(), ()> { Ok(()) }
    };
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[tokio::test]
        async fn foo () -> Result<(), ()> {
            serial_test::async_serial_core_with_return("", || async {
                { Ok(()) }
            }).await;
        }
    };
    assert_eq!(format!("{}", compare), format!("{}", stream));
}

#[test]
fn test_serial_async_return_reactor() {
    use quote::TokenStreamExt;
    let mut attrs = proc_macro2::TokenStream::new();
    attrs.append(syn::parse_str::<proc_macro2::Ident>("key").unwrap());
    let input = quote! {
        async fn foo() -> Result<(), ()> { Ok(()) }
    };
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[test]
        fn foo () -> Result<(), ()> {
            actix_rt::System::new("test").block_on (
                serial_test::async_serial_core_with_return("key", async {
                    { Ok(()) }
                })
            )
        }
    };
    assert_eq!(format!("{}", compare), format!("{}", stream));
}

#[test]
fn test_serial_async_reactor() {
    use quote::TokenStreamExt;
    let mut attrs = proc_macro2::TokenStream::new();
    attrs.append(syn::parse_str::<proc_macro2::Ident>("key").unwrap());
    let input = quote! {
        async fn foo() { () }
    };
    let stream = serial_core(attrs.into(), input);
    let compare = quote! {
        #[test]
        fn foo () {
            actix_rt::System::new("test").block_on (
                serial_test::async_serial_core("key", async {
                    { () }
                })
            )
        }
    };
    assert_eq!(format!("{}", compare), format!("{}", stream));
}
