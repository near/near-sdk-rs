use std::collections::HashMap;

use schemars::schema::{RootSchema, Schema};
use serde::{Deserialize, Serialize};

/// Current version of the ABI schema format.
const ABI_SCHEMA_SEMVER: &str = "0.1.0";

pub type TypeId = u32;

/// Contract ABI.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiRoot {
    /// Semver of the ABI schema format.
    pub abi_schema_version: String,
    /// Meta information about the contract.
    pub metainfo: AbiMetainfo,
    /// Core ABI information (functions and types).
    pub abi: Abi,
}

impl AbiRoot {
    pub fn new(abi: Abi) -> Self {
        Self {
            abi_schema_version: ABI_SCHEMA_SEMVER.to_string(),
            metainfo: Default::default(),
            abi,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct AbiMetainfo {
    /// The name of the smart contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,
    /// The version of the smart contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub version: Option<String>,
    /// The authors of the smart contract.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub authors: Vec<String>,
    /// Other arbitrary meta information.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    #[serde(flatten)]
    pub other: HashMap<String, String>,
}

/// Core ABI information.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Abi {
    /// ABIs of all contract's functions.
    pub functions: Vec<AbiFunction>,
    /// Type registry that maps type identifiers to JSON Schemas.
    pub types: Vec<AbiType>,
    /// Root JSON Schema containing all types referenced by the registry.
    pub root_schema: RootSchema,
}

/// Metadata of a single function.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiFunction {
    pub name: String,
    /// Whether function does not modify the state.
    pub is_view: bool,
    /// Whether function can be used to initialize the state.
    pub is_init: bool,
    /// Type identifiers of the function parameters.
    pub params: Vec<AbiParameter>,
    /// Type identifiers of the callbacks of the function.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub callbacks: Vec<AbiParameter>,
    /// Type identifier of the vararg callbacks of the function.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub callbacks_vec: Option<AbiParameter>,
    /// Return type identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub result: Option<AbiParameter>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiType {
    pub id: TypeId,
    pub schema: Schema,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiParameter {
    pub type_id: TypeId,
    pub serialization_type: AbiSerializerType,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AbiSerializerType {
    Json,
    Borsh,
}
