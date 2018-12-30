extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree};
use quote::quote;
use syn;

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
