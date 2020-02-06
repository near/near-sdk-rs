use crate::info_extractor::SerializerType;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, Error};

pub struct SerializerAttr {
    #[allow(dead_code)]
    paren_token: syn::token::Paren,
    pub serializer_type: SerializerType,
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

//#[cfg(test)]
//mod tests {
//    use super::parse_args;
//    use quote::quote;
//    use syn::ImplItemMethod;
//
//    #[test]
//    fn standard() {
//        let method: ImplItemMethod = syn::parse2(quote! {
//            #[callback_args_vec(arg0)]
//            fn simple_function() {
//            }
//        })
//        .unwrap();
//        let actual = syn::parse2(method.attrs[0].tokens.clone()).unwrap();
//        let expected = "arg0".to_string();
//        assert_eq!(actual, expected);
//    }
//}
