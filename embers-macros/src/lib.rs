use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Error, Expr, ExprLit, Ident, Lit, Meta, MetaNameValue, parse_macro_input};

/// Implements `Eq`, `Hash`, and `PartialEq` for a struct
/// based on a specific field that serves as its identity.
#[proc_macro_attribute]
pub fn identify(attr: TokenStream, item: TokenStream) -> TokenStream {
    let identifier_attr = parse_macro_input!(attr as Meta);
    let input = parse_macro_input!(item as DeriveInput);
    let field_ident = match identifier_attr {
        Meta::Path(path) => path.get_ident().cloned().unwrap(),
        Meta::List(list) => {
            let mut field_ident = None;
            list.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    field_ident = Some(ident.clone());
                }
                Err(meta.error("Expected an identifier"))
            })
            .unwrap();
            field_ident.expect("Expected an identifier")
        }
        Meta::NameValue(_value) => panic!("Expected an identifier"),
    };
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    TokenStream::from(quote! {
        #input
        impl #impl_generics std::cmp::PartialEq for #struct_name #ty_generics #where_clause {
            fn eq(&self, other: &Self) -> bool {
                self.#field_ident == other.#field_ident
            }
        }
        impl #impl_generics std::cmp::Eq for #struct_name #ty_generics #where_clause {}
        impl #impl_generics std::hash::Hash for #struct_name #ty_generics #where_clause {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.#field_ident.hash(state);
            }
        }
    })
}

#[proc_macro_derive(TypeKey, attributes(type_key))]
pub fn derive_type_key(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let mut type_key = None;
    for attr in &input.attrs {
        if attr.path().is_ident("type_key") {
            if let Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }),
                ..
            }) = &attr.meta
            {
                type_key = Some(lit_str.value());
                break;
            }
        }
    }
    let Some(type_key) = type_key else {
        return Error::new_spanned(name, "missing required attribute `type_key`")
            .to_compile_error()
            .into();
    };
    let embers = match crate_name("embers") {
        Ok(FoundCrate::Itself) => quote! { crate },
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote! { ::#ident }
        }
        Err(_error) => quote! { ::embers },
    };
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let expanded = quote! {
        impl #impl_generics #embers::utils::TypeKey for #name #ty_generics #where_clause {
            fn key() -> &'static #embers::utils::NamespacedKey {
                static KEY: std::sync::LazyLock<#embers::utils::NamespacedKey> =
                    std::sync::LazyLock::new(|| #type_key.parse::<#embers::utils::NamespacedKey>().unwrap());
                &*KEY
            }
        }
    };
    TokenStream::from(expanded)
}
