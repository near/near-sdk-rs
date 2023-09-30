use super::SerializerType;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{bracketed, parenthesized, Error};

pub struct SerializerAttr {
    #[allow(dead_code)]
    pub serializer_type: SerializerType,
}

impl Parse for SerializerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = if input.peek(syn::Token![#]) {
            let _: syn::Token![#] = input.parse().unwrap();
            let bcontent;
            let _ = bracketed!(bcontent in input);
            let _: Ident = bcontent.parse().unwrap();
            let content;
            let _ = parenthesized!(content in bcontent);
            let ident: Ident = content.parse().unwrap();
            ident
        } else {
            let _: Ident = input.parse().unwrap();
            let content;
            let _ = parenthesized!(content in input);
            let ident: Ident = content.parse().unwrap();
            ident
        };

        let serializer_type = match ident.to_string().as_str() {
            "borsh" => SerializerType::Borsh,
            "json" => SerializerType::JSON,
            _ => return Err(Error::new(input.span(), "Unsupported serializer type.")),
        };
        Ok(Self { serializer_type })
    }
}
