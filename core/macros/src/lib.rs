use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Event)]
pub fn event_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Ensure the input is an enum
    let enum_name = &input.ident;
    let data = match &input.data {
        Data::Enum(data_enum) => data_enum,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "The #[derive(Event)] macro can only be used on enums.",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate serde attributes for each variant
    let variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        let renamed_variant = format!("{}::{}", enum_name, variant_name);

        quote! {
            #[serde(rename = #renamed_variant)]
            #variant
        }
    });

    let tagged_enum = quote! {
        enum #enum_name {
            #(#variants),*
        }
    };

    TokenStream::from(tagged_enum)
}
