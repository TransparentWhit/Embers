use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Meta, parse_macro_input};

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
        Meta::NameValue(_) => panic!("Expected an identifier"),
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
