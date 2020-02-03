use crate::SerializerType;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Token;
use syn::{Attribute, Error};

pub struct SerializerAttr {
    #[allow(dead_code)]
    paren_token: token::Paren,
    serializer_type: SerializerType,
}

impl Parse for SerializerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = parenthesized!(content in input);
        let ident: Ident = content.parse()?;
        let serializer_type = match ident.to_string().as_str() {
            "borsh" => SerializerType::Borsh,
            "json" => SerializerType::JSON,
            _ => return Err(Error::new(input.span(), "Unsupported serializer type.")),
        };
        Ok(Self { paren_token, serializer_type })
    }
}

/// Parses attributes `#[serializer(borsh)]`.
pub fn parse_args(method: &Attribute) -> syn::Result<String> {
    let parsed: SerializerAttr = syn::parse2(attr.tokens.clone())?;
    Ok(parsed.args_vec.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use quote::quote;
    use syn::ImplItemMethod;

    #[test]
    fn standard() {
        let method: ImplItemMethod = syn::parse2(quote! {
            #[callback_args_vec(arg0)]
            fn simple_function() {
            }
        })
        .unwrap();
        let actual = parse_args(&method.attrs[0]).unwrap();
        let expected = "arg0".to_string();
        assert_eq!(actual, expected);
    }
}
