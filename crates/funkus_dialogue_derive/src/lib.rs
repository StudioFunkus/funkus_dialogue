//! Derive macro for dialogue resource registration.

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitStr, parse_macro_input};

#[proc_macro_derive(DialogueResource, attributes(dialogue))]
pub fn derive_dialogue_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let key = extract_key(&input);
    let ident = &input.ident;

    let impl_block = if let Some(key) = key {
        quote! {
            impl ::funkus_dialogue_core::DialogueResource for #ident {
                fn resource_key() -> &'static str {
                    #key
                }
            }
        }
    } else {
        quote! {
            impl ::funkus_dialogue_core::DialogueResource for #ident {}
        }
    };

    TokenStream::from(quote! {
        #impl_block
    })
}

fn extract_key(input: &DeriveInput) -> Option<String> {
    for attr in &input.attrs {
        if !attr.path().is_ident("dialogue") {
            continue;
        }
        let mut found = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("key") {
                let lit: LitStr = meta.value()?.parse()?;
                found = Some(lit.value());
            }
            Ok(())
        });
        if found.is_some() {
            return found;
        }
    }
    None
}
