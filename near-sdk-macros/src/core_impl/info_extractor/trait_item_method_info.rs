use super::AttrSigInfo;
use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{Error, LitStr, TraitItemMethod};

/// Information extracted from trait method.
pub struct TraitItemMethodInfo {
    /// Attributes and signature information.
    pub attr_sig_info: AttrSigInfo,
    /// The original AST of the trait item method.
    pub original: TraitItemMethod,
    /// String representation of method name, e.g. `"my_method"`.
    pub ident_byte_str: LitStr,
}

impl TraitItemMethodInfo {
    pub fn new(original: &mut TraitItemMethod) -> syn::Result<Self> {
        if original.default.is_some() {
            return Err(Error::new(
                original.span(),
                "Traits that are used to describe external contract should not include\
                 default implementations because this is not a valid use case of traits\
                 to describe external contracts.",
            ));
        }

        let TraitItemMethod { attrs, sig, .. } = original;

        let attr_sig_info = AttrSigInfo::new(attrs, sig)?;

        let ident_byte_str = LitStr::new(&attr_sig_info.ident.to_string(), Span::call_site());

        Ok(Self { attr_sig_info, original: original.clone(), ident_byte_str })
    }
}
