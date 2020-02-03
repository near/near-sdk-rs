use syn::export::{Span, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, token, Attribute, Error, Ident, ImplItemMethod, Token};

pub struct CallbackArgs {
    #[allow(dead_code)]
    paren_token: token::Paren,
    args: Punctuated<Ident, Token![,]>,
}

/// Parses attributes `#[callback_args(arg0, arg1)]`.
impl Parse for CallbackArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren_token: parenthesized!(content in input),
            args: content.parse_terminated(Ident::parse)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use crate::callback_args::CallbackArgs;
    use quote::quote;
    use syn::ImplItemMethod;

    #[test]
    fn standard() {
        let mut method: ImplItemMethod = syn::parse2(quote! {
            #[callback_args(arg0, arg1)]
            fn simple_function() {
            }
        })
        .unwrap();
        let actual: CallbackArgs = syn::parse2(method.attrs[0].tokens.clone()).unwrap();
        let actual: Vec<_> = actual.args.iter().map(|a| a.to_string()).collect();
        let expected = vec!["arg0".to_string(), "arg1".to_string()];
        assert_eq!(actual, expected);
    }

    #[test]
    fn one_arg() {
        let mut method: ImplItemMethod = syn::parse2(quote! {
            #[callback_args(arg0)]
            fn simple_function() {

            }
        })
        .unwrap();
        let actual: CallbackArgs = syn::parse2(method.attrs[0].tokens.clone()).unwrap();
        let actual: Vec<_> = actual.args.iter().map(|a| a.to_string()).collect();
        let expected = vec!["arg0".to_string()];
        assert_eq!(actual, expected);
    }
}
