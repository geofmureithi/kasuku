use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(FromSelect)]
pub fn from_select(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct name and field names
    let struct_name = &input.ident;

    let fields = match input.data {
        syn::Data::Struct(input) => {
            let fields: Vec<_> = input.fields.iter().map(|field| {
                let field_name = &field.ident;
                quote! {
                    #field_name: {
                        let field_index = labels.iter().position(|label| label.to_lowercase() == stringify!(#field_name).to_lowercase());
                        field_index.and_then(|index| {
                            row.get(index).cloned().unwrap().try_into().ok()
                        })
                        .unwrap_or_else(|| panic!("Failed to find value for field: {}", stringify!(#field_name)))
                    },
                }
            }).collect();
            fields
        }
        _ => panic!("FromSelect only works with structs"),
    };

    // Generate the implementation of the From<Select> trait
    let expanded = quote! {
        impl std::convert::TryFrom<context::payload::Row> for #struct_name {
            type Error = types::Error;
            fn try_from(payload: context::payload::Row) -> Result<Self, Self::Error> {
                let labels = payload.0;
                let row = payload.1;
                let result = #struct_name {
                    #(
                        #fields
                    )*
                };
                Ok(result)
            }
        }
    };

    // Return the generated code as a TokenStream
    TokenStream::from(expanded)
}
