mod serializer_attr;
pub use serializer_attr::SerializerAttr;

mod arg_info;
pub use arg_info::{ArgInfo, BindgenArgType};

mod attr_sig_info;
pub use attr_sig_info::AttrSigInfo;

mod impl_item_method_info;
pub use impl_item_method_info::ImplItemMethodInfo;

/// Type of serialization we use.
#[derive(PartialEq, Eq)]
pub enum SerializerType {
    JSON,
    Borsh,
}
