use crate::core_impl::info_extractor::AttrSigInfo;
use syn::{spanned::Spanned, Error, ImplItemMethod, Signature, Type, Visibility};

/// Information extracted from `ImplItemMethod`.
pub struct ImplItemMethodInfo {
    /// Information on the attributes and the signature of the method.
    pub attr_signature_info: AttrSigInfo,
    /// The type of the contract struct.
    pub struct_type: Type,
}

impl ImplItemMethodInfo {
    fn check_sig_modifiers(sig: &Signature) -> syn::Result<()> {
        if sig.asyncness.is_some() {
            return Err(Error::new(sig.span(), "Contract API is not allowed to be async."));
        }
        if sig.abi.is_some() {
            return Err(Error::new(
                sig.span(),
                "Contract API is not allowed to have binary interface.",
            ));
        }
        if sig.variadic.is_some() {
            return Err(Error::new(
                sig.span(),
                "Contract API is not allowed to have variadic arguments.",
            ));
        }

        Ok(())
    }

    /// Process the method and extract information important for near-sdk.
    pub fn new(
        original: &mut ImplItemMethod,
        is_trait_impl: bool,
        struct_type: Type,
    ) -> syn::Result<Option<Self>> {
        let ImplItemMethod { attrs, sig, .. } = original;
        Self::check_sig_modifiers(sig)?;
        if is_trait_impl || matches!(original.vis, Visibility::Public(_)) {
            let attr_signature_info = AttrSigInfo::new(attrs, sig)?;
            Ok(Some(Self { attr_signature_info, struct_type }))
        } else {
            Ok(None)
        }
    }
}
