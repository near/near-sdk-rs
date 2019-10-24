use syn::export::{Span, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, token, Error, Ident, ImplItemMethod};

pub struct CallbackArgsVec {
    #[allow(dead_code)]
    paren_token: token::Paren,
    args_vec: Ident,
}

impl Parse for CallbackArgsVec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self { paren_token: parenthesized!(content in input), args_vec: content.parse()? })
    }
}

/// Parses attributes searching for `#[callback_args_vec(argsvec)]` marker attribute.
/// If found returns an identity `argsvec`.
pub fn parse_args(method: &ImplItemMethod) -> syn::Result<Option<String>> {
    let attributes = &method.attrs;
    let mut res = None;
    for attr in attributes {
        if attr.path.to_token_stream().to_string().as_str() == "callback_args_vec" {
            if res.is_some() {
                return Err(Error::new(
                    Span::call_site(),
                    "Only one #[callback_args_vec(...)] attribute is allowed per method.",
                ));
            }

            let parsed: CallbackArgsVec = syn::parse2(attr.tokens.clone())?;
            res = Some(parsed.args_vec.to_string());
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use quote::quote;
    use syn::ImplItemMethod;

    #[test]
    fn standard() {
        let method: ImplItemMethod = syn::parse2(quote! {
            #[some_attribute0 tokens ? & hello]
            #[callback_args_vec(arg0)]
            #[some_attribute1 tokens ->]
            fn simple_function() {
            }
        })
        .unwrap();
        let actual = parse_args(&method).unwrap();
        let expected = Some("arg0".to_string());
        assert_eq!(actual, expected);
    }
}
