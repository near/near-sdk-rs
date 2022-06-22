use schemars::schema::{RootSchema, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current version of the ABI schema format.
const ABI_SCHEMA_SEMVER: &str = "0.1.0";

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

    pub fn combine<I: IntoIterator<Item = AbiRoot>>(abi_roots: I) -> AbiRoot {
        let mut abi_schema_version = "".to_string();
        let mut abis = Vec::<Abi>::new();

        for abi_root in abi_roots {
            // Set common schema version to the current schema version. Only happens with the
            // first element.
            if abi_schema_version.is_empty() {
                abi_schema_version = abi_root.abi_schema_version.clone();
            }

            // Check that all ABIs conform to the same version
            if abi_root.abi_schema_version != abi_schema_version {
                panic!(
                    "Conflicting ABI schema versions: {} and {}",
                    &abi_root.abi_schema_version, abi_schema_version
                );
            }

            // Check that all metadata is empty.
            if !abi_root.metadata.name.is_none()
                || !abi_root.metadata.version.is_none()
                || !abi_root.metadata.authors.is_empty()
                || !abi_root.metadata.other.is_empty()
            {
                panic!("Non-empty metadata: {:?}", &abi_root.metadata);
            }

            abis.push(abi_root.abi);
        }

        let abi = Abi::combine(abis);
        AbiRoot { abi_schema_version, metadata: AbiMetadata::default(), abi }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct AbiMetadata {
    /// The name of the smart contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The version of the smart contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The authors of the smart contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// Other arbitrary metadata.
    #[serde(default, flatten, skip_serializing_if = "HashMap::is_empty")]
    pub other: HashMap<String, String>,
}

/// Core ABI information.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Abi {
    /// ABIs of all contract's functions.
    pub functions: Vec<AbiFunction>,
    /// Root JSON Schema containing all types referenced in the functions.
    pub root_schema: RootSchema,
}

impl Abi {
    pub fn combine<I: IntoIterator<Item = Abi>>(abis: I) -> Abi {
        let mut functions = Vec::<AbiFunction>::new();
        let mut gen = schemars::gen::SchemaGenerator::default();
        let definitions = gen.definitions_mut();

        for abi in abis {
            // Update resulting JSON Schema
            definitions.extend(abi.root_schema.definitions.to_owned());

            // Update resulting function list
            functions.extend(abi.functions);
        }
        // Sort the function list for readability
        functions.sort_by(|x, y| x.name.cmp(&y.name));

        Abi { functions, root_schema: gen.into_root_schema_for::<String>() }
    }
}

/// ABI of a single function.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiFunction {
    pub name: String,
    /// Whether function does not modify the state.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_view: bool,
    /// Whether function can be used to initialize the state.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_init: bool,
    /// Whether function is accepting $NEAR.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_payable: bool,
    /// Whether function can only accept calls from self (current account).
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_private: bool,
    /// Type identifiers of the function parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<AbiParameter>,
    /// Type identifiers of the callbacks of the function.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callbacks: Vec<AbiType>,
    /// Type identifier of the vararg callbacks of the function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callbacks_vec: Option<AbiType>,
    /// Return type identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<AbiType>,
}

/// Information about a single named function parameter.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiParameter {
    /// Parameter name (e.g. `p1` in `fn foo(p1: u32) {}`).
    pub name: String,
    /// JSON Subschema representing the type of the parameter.
    pub type_schema: Schema,
    /// How the parameter is serialized (either JSON or Borsh).
    pub serialization_type: AbiSerializationType,
}

/// Information about a single type (e.g. return type).
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AbiType {
    /// JSON Subschema that represents this type.
    pub type_schema: Schema,
    /// How the type instance is serialized (either JSON or Borsh).
    pub serialization_type: AbiSerializationType,
}

/// Represents how instances of a certain type are serialized in a certain context. Same type
/// can have different serialization types associated with it depending on where they occur.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AbiSerializationType {
    Json,
    Borsh,
}

fn is_false(b: &bool) -> bool {
    !b
}
