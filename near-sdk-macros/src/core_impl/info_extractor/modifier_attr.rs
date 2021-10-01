use proc_macro2::Ident;
use syn::parenthesized;
use syn::parse::{Parse, ParseStream};

pub struct ModifierAttr {
    #[allow(dead_code)]
    paren_token: syn::token::Paren,
    pub modifier: Ident,
}

impl Parse for ModifierAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = parenthesized!(content in input);
        let modifier: Ident = content.parse()?;

        Ok(Self { paren_token, modifier })
    }
}
