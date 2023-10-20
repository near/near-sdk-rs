use syn::{Receiver, ReturnType, Type};

mod serializer_attr;
pub use serializer_attr::SerializerAttr;

mod arg_info;
pub use arg_info::{ArgInfo, BindgenArgType};

mod handle_result_attr;
pub use handle_result_attr::HandleResultAttr;

mod attr_sig_info;
pub use attr_sig_info::AttrSigInfo;

mod impl_item_method_info;
pub use impl_item_method_info::ImplItemMethodInfo;

mod trait_item_method_info;
pub use trait_item_method_info::*;

mod item_trait_info;
pub use item_trait_info::ItemTraitInfo;

mod item_impl_info;

mod init_attr;
pub use init_attr::InitAttr;

mod visitor;

pub use item_impl_info::ItemImplInfo;

/// Type of serialization we use.
#[derive(Clone, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum SerializerType {
    JSON,
    Borsh,
}

#[derive(Clone, PartialEq, Eq)]
pub enum MethodKind {
    Call(CallMethod),
    View(ViewMethod),
    Init(InitMethod),
}

#[derive(Clone, PartialEq, Eq)]
pub struct CallMethod {
    /// Whether method accepting $NEAR.
    pub is_payable: bool,
    /// Whether method can accept calls from self (current account)
    pub is_private: bool,
    /// The serializer that we use for the return type.
    pub result_serializer: SerializerType,
    /// The receiver, like `mut self`, `self`, `&mut self`, `&self`, or `None`.
    pub receiver: Option<Receiver>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ViewMethod {
    /// Whether method can accept calls from self (current account)
    pub is_private: bool,
    /// The serializer that we use for the return type.
    pub result_serializer: SerializerType,
    /// The receiver, like `mut self`, `self`, `&mut self`, `&self`, or `None`.
    pub receiver: Option<Receiver>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct InitMethod {
    /// Whether method accepting $NEAR.
    pub is_payable: bool,
    /// Whether init method ignores state
    pub ignores_state: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Returns {
    /// The original return type of the method in the Rust AST.
    pub original: ReturnType,
    /// The return type of the method in our logic.
    pub kind: ReturnKind,
}

#[derive(Clone, PartialEq, Eq)]
pub enum ReturnKind {
    Default,
    General(Type),
    HandlesResult(Type),
}
