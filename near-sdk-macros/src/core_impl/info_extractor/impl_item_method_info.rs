use crate::core_impl::info_extractor::AttrSigInfo;
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
        let attr_signature_info = AttrSigInfo::new(attrs, sig)?;
        let is_public = matches!(original.vis, Visibility::Public(_));
        Ok(Self { attr_signature_info, is_public, struct_type })
    }
}

// Rustfmt removes comas.
#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use syn::{parse_quote, Type, ImplItemMethod};
    use crate::core_impl::ImplItemMethodInfo;

    #[test]
    fn init_no_return() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[init]
            pub fn method(k: &mut u64) { }
        };
        let actual = ImplItemMethodInfo::new(&mut method, impl_type).map(|_| ()).unwrap_err();
        let expected = "Init function must return the contract state.";
        assert_eq!(expected, actual.to_string());
    }

    #[test]
    fn handle_result_incorrect_return_type() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            #[handle_result]
            pub fn method(&self) -> &'static str { }
        };
        let actual = ImplItemMethodInfo::new(&mut method, impl_type).map(|_| ()).unwrap_err();
        let expected = "Function marked with #[handle_result] should return Result<T, E> (where E implements FunctionError).";
        assert_eq!(expected, actual.to_string());
    }

    #[test]
    fn handle_result_without_marker() {
        let impl_type: Type = syn::parse_str("Hello").unwrap();
        let mut method: ImplItemMethod = parse_quote! {
            pub fn method(&self) -> Result<u64, &'static str> { }
        };
        let actual = ImplItemMethodInfo::new(&mut method, impl_type).map(|_| ()).unwrap_err();
        let expected = "Serializing Result<T, E> has been deprecated. Consider marking your method with #[handle_result] if the second generic represents a panicable error or replacing Result with another two type sum enum otherwise. If you really want to keep the legacy behavior, mark the method with #[handle_result] and make it return Result<Result<T, E>, near_sdk::Abort>.";
        assert_eq!(expected, actual.to_string());
    }
}
