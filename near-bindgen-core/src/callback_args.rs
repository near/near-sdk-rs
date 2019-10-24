use syn::export::{Span, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, token, Error, Ident, ImplItemMethod, Token};

pub struct CallbackArgs {
    #[allow(dead_code)]
    paren_token: token::Paren,
    args: Punctuated<Ident, Token![,]>,
}

impl Parse for CallbackArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren_token: parenthesized!(content in input),
            args: content.parse_terminated(Ident::parse)?,
        })
    }
}

/// Parses attributes searching for `#[callback_args(arg0, arg1)]` marker attribute.
/// If found returns a vector of identities `arg0`, `arg1`, etc.
pub fn parse_args(method: &ImplItemMethod) -> syn::Result<Option<Vec<String>>> {
    let attributes = &method.attrs;
    let mut res = None;
    for attr in attributes {
        if attr.path.to_token_stream().to_string().as_str() == "callback_args" {
            if res.is_some() {
                return Err(Error::new(
                    Span::call_site(),
                    "Only one #[callback_args(...)] attribute is allowed per method.",
                ));
            }

            let parsed: CallbackArgs = syn::parse2(attr.tokens.clone())?;
            if parsed.args.is_empty() {
                return Err(Error::new(
                    Span::call_site(),
                    "#[callback_args(...)] should use at least one argument.",
                ));
            }
            res = Some(parsed.args.iter().map(|arg| arg.to_string()).collect());
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
            #[callback_args(arg0, arg1)]
            #[some_attribute1 tokens ->]
            fn simple_function() {
            }
        })
        .unwrap();
        let actual = parse_args(&method).unwrap();
        let expected = Some(vec!["arg0".to_string(), "arg1".to_string()]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn one_arg() {
        let method: ImplItemMethod = syn::parse2(quote! {
            #[callback_args(arg0)]
            fn simple_function() {

            }
        })
        .unwrap();
        let actual = parse_args(&method).unwrap();
        let expected = Some(vec!["arg0".to_string()]);
        assert_eq!(actual, expected);
    }
}
