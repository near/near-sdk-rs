use super::AttrSigInfo;
use crate::core_impl::utils;
use proc_macro2::TokenStream as TokenStream2;
use syn::spanned::Spanned;
use syn::{Error, LitStr, TraitItemFn};

/// Information extracted from trait method.
pub struct TraitItemMethodInfo {
    /// Attributes and signature information.
    pub attr_sig_info: AttrSigInfo,
    /// The original AST of the trait item method.
    #[allow(unused)]
    pub original: TraitItemFn,
    /// String representation of method name, e.g. `"my_method"`.
    #[allow(unused)]
    pub ident_byte_str: LitStr,
}

impl TraitItemMethodInfo {
    pub fn new(original: &mut TraitItemFn, trait_name: &TokenStream2) -> syn::Result<Self> {
        if original.default.is_some() {
            return Err(Error::new(
                original.span(),
                "Traits that are used to describe external contract should not include\
                 default implementations because this is not a valid use case of traits\
                 to describe external contracts.",
            ));
        }

        let TraitItemFn { attrs, sig, .. } = original;

        utils::sig_is_supported(sig)?;
        let attr_sig_info = AttrSigInfo::new(attrs, sig, trait_name)?;

        let ident_byte_str =
            LitStr::new(&attr_sig_info.ident.to_string(), attr_sig_info.ident.span());

        Ok(Self { attr_sig_info, original: original.clone(), ident_byte_str })
    }
}
