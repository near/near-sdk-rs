use syn::{ImplItemMethod, Type, Visibility};

mod serializer_attr;
pub use serializer_attr::SerializerAttr;

mod arg_info;
pub use arg_info::{ArgInfo, BindgenArgType};

mod signature_info;
pub use signature_info::AttrSignatureInfo;

/// Type of serialization we use.
#[derive(PartialEq, Eq)]
pub enum SerializerType {
    JSON,
    Borsh,
}

pub struct ImplMethodInfo {
    /// Information on the attributes and the signature of the method.
    pub attr_signature_info: AttrSignatureInfo,
    /// Whether method has `pub` modifier or a part of trait implementation.
    pub is_public: bool,
    /// The original code of the method.
    pub original: ImplItemMethod,
    /// The type of the contract struct.
    pub struct_type: Type,
}

impl ImplMethodInfo {
    /// Process the method and extract information important for near-bindgen.
    pub fn new(
        original: ImplItemMethod,
        struct_type: Type,
        is_trait_impl: bool,
    ) -> syn::Result<Self> {
        let attr_signature_info =
            AttrSignatureInfo::new(original.attrs.clone(), original.sig.clone())?;
        let is_public = match original.vis {
            Visibility::Public(_) => true,
            _ => is_trait_impl,
        };
        Ok(Self { attr_signature_info, is_public, original, struct_type })
    }
}
