extern crate proc_macro;

use proc_macro::TokenStream;
use std::str::FromStr;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Ident};


///
/// custom attribute macro
#[proc_macro_attribute]
pub fn inspect_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
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
    // TokenStream::from()
    let struct_name = format_ident!("{}", input.ident);
    // let raw_input: syn::Expr = syn::parse_str(raw.as_str()).expect("unable to parse");

    let expanded = quote!{
        #input
        impl Debug for #struct_name{
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "test")
            }
        }
    };
    println!("expanded >> {}", expanded);


    TokenStream::from(expanded)
    // TokenStream::from_str(raw.as_str()).unwrap()


}


///
/// derive macro
#[proc_macro_derive(MyTrait)]
pub fn my_trait_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let ast = parse_macro_input!(input as DeriveInput);

    // Get the name of the struct or enum the derive macro is applied to
    let name = &ast.ident;

    // Generate the code for implementing the trait (replace MyTrait with your trait name)
    let gen = quote! {
        impl MyTrait for #name {
            // Implement trait methods or other code here
            // For example:
            fn bark(&self)->String{
                println!("bark>>>>");
                String::from("hello bark")
            }
        }
    };

    // Return the generated code as a TokenStream
    gen.into()
}

use syn::{ LitInt};



///
/// function-like macro
#[proc_macro]
pub fn increment(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as LitInt);

    // Extract the value of the integer literal
    let value = input.base10_parse::<u64>().unwrap();

    // Increment the value
    let incremented_value = value + 1;

    // Generate the new token stream with the incremented value
    let output = quote! {
        #incremented_value
    };

    output.into()
}