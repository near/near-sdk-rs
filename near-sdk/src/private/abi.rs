use std::collections::HashMap;

use schemars::schema::{RootSchema, Schema};
use serde::{Deserialize, Serialize};

/// Current version of the ABI schema format.
const ABI_SCHEMA_SEMVER: &str = "0.1.0";

/// NEAR ABI does not use Rust types, instead it uses abstract type identifiers (represented by
/// `TypeId`) that are then mapped to the corresponding JSON subschema.
pub type TypeId = u32;

/// Contract ABI.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiRoot {
    /// Semver of the ABI schema format.
    pub abi_schema_version: String,
    /// Metadata information about the contract.
    pub metadata: AbiMetadata,
    /// Core ABI information (functions and types).
    pub abi: Abi,
}

impl AbiRoot {
    pub fn new(abi: Abi) -> Self {
        Self {
            abi_schema_version: ABI_SCHEMA_SEMVER.to_string(),
            metadata: Default::default(),
            abi,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct AbiMetadata {
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
    /// Other arbitrary metadata.
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
    pub types: Vec<AbiTypeDef>,
    /// Root JSON Schema containing all types referenced by the registry.
    pub root_schema: RootSchema,
}

/// ABI of a single function.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiFunction {
    pub name: String,
    /// Whether function does not modify the state.
    pub is_view: bool,
    /// Whether function can be used to initialize the state.
    pub is_init: bool,
    /// Type identifiers of the function parameters.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub params: Vec<AbiParameter>,
    /// Type identifiers of the callbacks of the function.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub callbacks: Vec<AbiType>,
    /// Type identifier of the vararg callbacks of the function.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub callbacks_vec: Option<AbiType>,
    /// Return type identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub result: Option<AbiType>,
}

/// Mapping from [TypeId] to JSON subschema.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiTypeDef {
    pub id: TypeId,
    pub schema: Schema,
}

/// Information about a single named function parameter.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiParameter {
    /// Parameter name (e.g. `p1` in `fn foo(p1: u32) {}`).
    pub name: String,
    /// Identifier representing the type of the parameter (see [TypeId]).
    pub type_id: TypeId,
    /// How the parameter is serialized (either JSON or Borsh).
    pub serialization_type: AbiSerializerType,
}

/// Information about a single type (e.g. return type).
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiType {
    /// Identifier that represents this type (see [TypeId]).
    pub type_id: TypeId,
    /// How the type instance is serialized (either JSON or Borsh).
    pub serialization_type: AbiSerializerType,
}

/// Represents how instances of a certain type are serialized in a certain context. Same type
/// can have different serialization types associated with it depending on where they occur.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AbiSerializerType {
    Json,
    Borsh,
}
