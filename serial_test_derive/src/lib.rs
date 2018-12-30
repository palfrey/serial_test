extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree};
use quote::quote;
use syn;

#[proc_macro_attribute]
pub fn serial(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = attr.into_iter().collect::<Vec<TokenTree>>();
    if attrs.len() != 1 {
        panic!("Expected a single argument");
    }
    if let TokenTree::Ident(id) = &attrs[0] {
        let key = id.to_string();
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
    } else {
        panic!("Expected a single name as argument");
    }
}
