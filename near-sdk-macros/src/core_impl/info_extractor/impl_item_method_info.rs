use crate::core_impl::info_extractor::AttrSigInfo;
use crate::core_impl::utils;
use quote::ToTokens;
use syn::{ImplItemMethod, Type, Visibility};

/// Information extracted from `ImplItemMethod`.
pub struct ImplItemMethodInfo {
    /// Information on the attributes and the signature of the method.
    pub attr_signature_info: AttrSigInfo,
    /// The type of the contract struct.
    pub struct_type: Type,
}

impl ImplItemMethodInfo {
    /// Process the method and extract information important for near-sdk.
    pub fn new(
        original: &mut ImplItemMethod,
        is_trait_impl: bool,
        struct_type: Type,
    ) -> syn::Result<Option<Self>> {
        let ImplItemMethod { attrs, sig, .. } = original;
        utils::sig_is_supported(sig)?;
        if is_trait_impl || matches!(original.vis, Visibility::Public(_)) {
            let attr_signature_info = AttrSigInfo::new(attrs, sig, &struct_type.to_token_stream())?;
            Ok(Some(Self { attr_signature_info, struct_type }))
        } else {
            Ok(None)
        }
    }
}
