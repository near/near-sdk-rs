use syn::export::{Span, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, token, Attribute, Error, Ident, ImplItemMethod};

pub struct CallbackArgsVec {
    #[allow(dead_code)]
    paren_token: token::Paren,
    args_vec: Ident,
}

/// Parses attribute `#[callback_args_vec(argsvec)]`.
impl Parse for CallbackArgsVec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self { paren_token: parenthesized!(content in input), args_vec: content.parse()? })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use crate::callback_args_vec::CallbackArgsVec;
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
        let actual: CallbackArgsVec = syn::parse2(method.attrs[0].tokens.clone()).unwrap();
        let expected = "arg0".to_string();
        assert_eq!(actual.args_vec.to_string(), expected);
    }
}
