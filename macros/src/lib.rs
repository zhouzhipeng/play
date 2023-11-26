extern crate proc_macro;

use proc_macro::TokenStream;
use std::str::FromStr;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Ident};

#[proc_macro_attribute]
pub fn inspect_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    let raw = item.to_string();
    println!("test.>>");
    // Parse the input as a DeriveInput
    let input = parse_macro_input!(item as DeriveInput);

    // Get the parameter passed to the macro (if any)
    let param = if !attr.is_empty() {
        Some(attr.to_string())
    } else {
        None
    };

    // Extract fields from the struct
    let fields = match input.data {
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                &fields.named
            } else {
                unimplemented!("Only named fields are supported");
            }
        }
        _ => unimplemented!("Only structs with named fields are supported"),
    };

    // Create output based on the extracted information
    let field_names: Vec<String> = fields.iter().filter(|field|field.ident.is_some()).map(|field| field.ident.as_ref().unwrap().to_string()).collect();
    println!("fields >> {:?}", field_names);
    println!("Parameter: {:?}", param);

    // TokenStream::from(expanded)
    TokenStream::from_str(raw.as_str()).unwrap()


}
