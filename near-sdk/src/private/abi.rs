use std::collections::{BTreeMap, HashMap};

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

impl Abi {
    pub fn combine<I: IntoIterator<Item = Abi>>(abis: I) -> Abi {
        let mut functions = Vec::<AbiFunction>::new();
        let mut types = Vec::<AbiTypeDef>::new();
        let mut gen = schemars::gen::SchemaGenerator::default();

        let definitions = gen.definitions_mut();
        // Max used type id in the ABI model preceding the current one. 0 by default so we can't
        // reuse type id 0 in the resulting ABI, which is not a big deal.
        let mut max_type_id: TypeId = 0;
        for abi in abis {
            // Update resulting JSON Schema
            definitions.extend(abi.root_schema.definitions.to_owned());

            // Update resulting type mapping

            // Max used type id used in the current ABI model.
            let mut new_max_type_id: TypeId = 0;
            // Mapping from the old type id to the new type id.
            let mut mapping = BTreeMap::<TypeId, TypeId>::new();
            for typ in &abi.types {
                let new_type_id: TypeId = typ.id + max_type_id + 1;
                mapping.insert(typ.id, new_type_id);
                types.push(AbiTypeDef { id: new_type_id, schema: typ.schema.to_owned() });
                new_max_type_id = std::cmp::max(new_max_type_id, new_type_id);
            }
            max_type_id = new_max_type_id;

            // Update resulting function list
            for func in &abi.functions {
                let mut new_func = func.to_owned();
                new_func.params = new_func
                    .params
                    .iter()
                    .map(|p| {
                        let mut new_p = p.to_owned();
                        new_p.type_id = mapping[&p.type_id];
                        new_p
                    })
                    .collect();
                new_func.callbacks = new_func
                    .callbacks
                    .iter()
                    .map(|t| {
                        let mut new_t = t.to_owned();
                        new_t.type_id = mapping[&t.type_id];
                        new_t
                    })
                    .collect();
                new_func.callbacks_vec = new_func.callbacks_vec.map(|t| {
                    let mut new_t = t.to_owned();
                    new_t.type_id = mapping[&t.type_id];
                    new_t
                });
                new_func.result = new_func.result.map(|t| {
                    let mut new_t = t.to_owned();
                    new_t.type_id = mapping[&t.type_id];
                    new_t
                });

                functions.push(new_func);
            }
        }

        Abi { functions, types, root_schema: gen.into_root_schema_for::<String>() }
    }
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
