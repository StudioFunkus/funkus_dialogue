//! Derive macros for dialogue resource/message registration.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, LitStr, parse_macro_input};

#[proc_macro_derive(DialogueResource, attributes(dialogue))]
pub fn derive_dialogue_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let key = extract_key(&input);
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let register_fn = format_ident!("__funkus_dialogue_register_resource_for_{}", ident);

    let impl_block = if let Some(key) = key {
        quote! {
            impl #impl_generics ::funkus_dialogue_core::DialogueResource for #ident #ty_generics #where_clause {
                fn resource_key() -> &'static str {
                    #key
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics ::funkus_dialogue_core::DialogueResource for #ident #ty_generics #where_clause {}
        }
    };

    let registration_block = if input.generics.params.is_empty() {
        quote! {
            #[allow(non_snake_case)]
            #[doc(hidden)]
            fn #register_fn(type_registry: &mut ::bevy::reflect::TypeRegistry) {
                type_registry.register::<#ident #ty_generics>();
                type_registry
                    .register_type_data::<#ident #ty_generics, ::funkus_dialogue_core::DialogueResourceTypeData>();
            }

            ::funkus_dialogue_core::__private::inventory::submit! {
                ::funkus_dialogue_core::registry::DialogueResourceRegistration::new(#register_fn)
            }
        }
    } else {
        quote! {
            compile_error!(
                "`#[derive(DialogueResource)]` only supports concrete (non-generic) resource types."
            );
        }
    };

    TokenStream::from(quote! {
        #impl_block
        #registration_block
    })
}

#[proc_macro_derive(DialogueMessage, attributes(dialogue))]
pub fn derive_dialogue_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let key = extract_key(&input);
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let register_fn = format_ident!("__funkus_dialogue_register_message_for_{}", ident);
    let add_message_fn = format_ident!("__funkus_dialogue_add_message_for_{}", ident);

    let impl_block = if let Some(key) = key {
        quote! {
            impl #impl_generics ::funkus_dialogue_core::DialogueMessage for #ident #ty_generics #where_clause {
                fn message_key() -> &'static str {
                    #key
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics ::funkus_dialogue_core::DialogueMessage for #ident #ty_generics #where_clause {}
        }
    };

    let registration_block = if input.generics.params.is_empty() {
        quote! {
            #[allow(non_snake_case)]
            #[doc(hidden)]
            fn #register_fn(type_registry: &mut ::bevy::reflect::TypeRegistry) {
                type_registry.register::<#ident #ty_generics>();
                type_registry
                    .register_type_data::<#ident #ty_generics, ::funkus_dialogue_core::DialogueMessageTypeData>();
            }

            #[allow(non_snake_case)]
            #[doc(hidden)]
            fn #add_message_fn(app: &mut ::bevy::prelude::App) {
                app.add_message::<#ident #ty_generics>();
            }

            ::funkus_dialogue_core::__private::inventory::submit! {
                ::funkus_dialogue_core::registry::DialogueMessageRegistration::new(
                    #register_fn,
                    #add_message_fn,
                )
            }
        }
    } else {
        quote! {
            compile_error!(
                "`#[derive(DialogueMessage)]` only supports concrete (non-generic) message types."
            );
        }
    };

    TokenStream::from(quote! {
        #impl_block
        #registration_block
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
