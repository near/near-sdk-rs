use crate::core_impl::info_extractor::AttrSigInfo;
use quote::ToTokens;
use syn::{ImplItemMethod, Type, Visibility};

/// Information extracted from `ImplItemMethod`.
pub struct ImplItemMethodInfo {
    /// Information on the attributes and the signature of the method.
    pub attr_signature_info: AttrSigInfo,
    /// Whether method has `pub` modifier.
    pub is_public: bool,
    /// The type of the contract struct.
    pub struct_type: Type,
}

impl ImplItemMethodInfo {
    /// Process the method and extract information important for near-sdk.
    pub fn new(original: &mut ImplItemMethod, struct_type: Type) -> syn::Result<Self> {
        let ImplItemMethod { attrs, sig, .. } = original;
        let attr_signature_info = AttrSigInfo::new(attrs, sig, &struct_type.to_token_stream())?;
        let is_public = matches!(original.vis, Visibility::Public(_));
        Ok(Self { attr_signature_info, is_public, struct_type })
    }
}
