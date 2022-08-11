use borsh::schema::{BorshSchemaContainer, Declaration, Definition, Fields, VariantName};
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
    /// Human-readable documentation parsed from the source file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
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
    /// Parameter type along with its serialization type (e.g. `u32` and `borsh` in `fn foo(#[serializer(borsh)] p1: u32) {}`).
    #[serde(flatten)]
    pub typ: AbiType,
}

/// Information about a single type (e.g. return type).
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "serialization_type")]
#[serde(rename_all = "lowercase")]
pub enum AbiType {
    Json {
        /// JSON Subschema that represents this type (can be an inline primitive, a reference to the root schema and a few other corner-case things).
        type_schema: Schema,
    },
    Borsh {
        /// Inline Borsh schema that represents this type.
        #[serde(with = "BorshSchemaContainerDef")]
        type_schema: BorshSchemaContainer,
    },
}

// TODO: Maybe implement `Clone` for `BorshSchemaContainer` in borsh upstream?
impl Clone for AbiType {
    fn clone(&self) -> Self {
        match self {
            Self::Json { type_schema } => Self::Json { type_schema: type_schema.clone() },
            Self::Borsh { type_schema } => {
                let type_schema = BorshSchemaContainer {
                    declaration: type_schema.declaration.clone(),
                    definitions: type_schema
                        .definitions
                        .iter()
                        .map(|(k, v)| (k.clone(), borsh_clone::clone_definition(v)))
                        .collect(),
                };
                Self::Borsh { type_schema }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "BorshSchemaContainer")]
struct BorshSchemaContainerDef {
    declaration: Declaration,
    #[serde(with = "borsh_serde")]
    definitions: HashMap<Declaration, Definition>,
}

/// Cloning functions for borsh types.
mod borsh_clone {
    use borsh::schema::{Definition, Fields};

    pub fn clone_fields(fields: &Fields) -> Fields {
        match fields {
            Fields::Empty => Fields::Empty,
            Fields::NamedFields(f) => Fields::NamedFields(f.clone()),
            Fields::UnnamedFields(f) => Fields::UnnamedFields(f.clone()),
        }
    }

    pub fn clone_definition(definition: &Definition) -> Definition {
        match definition {
            Definition::Array { length, elements } => {
                Definition::Array { length: length.clone(), elements: elements.clone() }
            }
            Definition::Sequence { elements } => {
                Definition::Sequence { elements: elements.clone() }
            }
            Definition::Tuple { elements } => Definition::Tuple { elements: elements.clone() },
            Definition::Enum { variants } => Definition::Enum { variants: variants.clone() },
            Definition::Struct { fields } => Definition::Struct { fields: clone_fields(fields) },
        }
    }
}

/// This submodules follows https://serde.rs/remote-derive.html to derive Serialize/Deserialize for
/// `BorshSchemaContainer` parameters. The top-level serialization type is `HashMap<Declaration, Definition>`
/// for the sake of being easily plugged into `BorshSchemaContainerDef` (see its parameters).
mod borsh_serde {
    use super::*;
    use serde::ser::SerializeMap;
    use serde::{Deserializer, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "Definition")]
    enum DefinitionDef {
        Array {
            length: u32,
            elements: Declaration,
        },
        #[serde(with = "transparent")]
        Sequence {
            elements: Declaration,
        },
        #[serde(with = "transparent")]
        Tuple {
            elements: Vec<Declaration>,
        },
        #[serde(with = "transparent")]
        Enum {
            variants: Vec<(VariantName, Declaration)>,
        },
        #[serde(with = "transparent_fields")]
        Struct {
            fields: Fields,
        },
    }

    #[derive(Serialize, Deserialize)]
    struct HelperDefinition(#[serde(with = "DefinitionDef")] Definition);

    /// #[serde(transparent)] does not support enum variants, so we have to use a custom ser/de impls for now.
    /// See https://github.com/serde-rs/serde/issues/2092.
    mod transparent {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        pub fn serialize<T, S>(field: &T, serializer: S) -> Result<S::Ok, S::Error>
        where
            T: Serialize,
            S: Serializer,
        {
            serializer.serialize_some(&field)
        }

        pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
        where
            T: Deserialize<'de>,
            D: Deserializer<'de>,
        {
            T::deserialize(deserializer)
        }
    }

    /// Since `Fields` itself does not implement `Serialization`/`Deserialization`, we can't use
    /// `transparent` in combination with `#[serde(with = "...")]. Instead we have do it in this
    /// roundabout way.
    mod transparent_fields {
        use super::borsh_clone;
        use borsh::schema::{Declaration, FieldName, Fields};
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        #[derive(Serialize, Deserialize)]
        #[serde(remote = "Fields", untagged)]
        enum FieldsDef {
            NamedFields(Vec<(FieldName, Declaration)>),
            UnnamedFields(Vec<Declaration>),
            Empty,
        }

        #[derive(Serialize, Deserialize)]
        struct HelperFields(#[serde(with = "FieldsDef")] Fields);

        pub fn serialize<S>(fields: &Fields, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            HelperFields(borsh_clone::clone_fields(fields)).serialize(serializer)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Fields, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(HelperFields::deserialize(deserializer)?.0)
        }
    }

    pub fn serialize<S>(
        map: &HashMap<Declaration, Definition>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map_ser = serializer.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            map_ser.serialize_entry(k, &HelperDefinition(borsh_clone::clone_definition(v)))?;
        }
        map_ser.end()
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<Declaration, Definition>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = HashMap::<Declaration, HelperDefinition>::deserialize(deserializer)?;
        Ok(map.into_iter().map(|(k, HelperDefinition(v))| (k, v)).collect())
    }
}

fn is_false(b: &bool) -> bool {
    !b
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshSchema;
    use serde_json::Value;

    #[test]
    fn test_serde_array() {
        let abi_type = AbiType::Borsh { type_schema: <[u32; 2]>::schema_container() };
        let value = serde_json::to_value(&abi_type).unwrap();
        let expected_json = r#"
          {
            "serialization_type": "borsh",
            "type_schema": {
              "declaration": "Array<u32, 2>",
              "definitions": {
                "Array<u32, 2>": {
                  "Array": {
                    "length": 2,
                    "elements": "u32"
                  }
                }
              }
            }
          }
        "#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(value, expected_value);

        if let AbiType::Borsh { type_schema } = serde_json::from_str(expected_json).unwrap() {
            assert_eq!(type_schema.declaration, "Array<u32, 2>".to_string());
            assert_eq!(type_schema.definitions.len(), 1);
            assert_eq!(
                type_schema.definitions.get("Array<u32, 2>").unwrap(),
                &Definition::Array { length: 2, elements: "u32".to_string() }
            );
        } else {
            panic!("Unexpected serialization type")
        }
    }

    #[test]
    fn test_serde_sequence() {
        let abi_type = AbiType::Borsh { type_schema: <Vec<u32>>::schema_container() };
        let value = serde_json::to_value(&abi_type).unwrap();
        let expected_json = r#"
          {
            "serialization_type": "borsh",
            "type_schema": {
              "declaration": "Vec<u32>",
              "definitions": {
                "Vec<u32>": {
                  "Sequence": "u32"
                }
              }
            }
          }
        "#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(value, expected_value);

        if let AbiType::Borsh { type_schema } = serde_json::from_str(expected_json).unwrap() {
            assert_eq!(type_schema.declaration, "Vec<u32>".to_string());
            assert_eq!(type_schema.definitions.len(), 1);
            assert_eq!(
                type_schema.definitions.get("Vec<u32>").unwrap(),
                &Definition::Sequence { elements: "u32".to_string() }
            );
        } else {
            panic!("Unexpected serialization type")
        }
    }

    #[test]
    fn test_serde_tuple() {
        let abi_type = AbiType::Borsh { type_schema: <(u32, u32)>::schema_container() };
        let value = serde_json::to_value(&abi_type).unwrap();
        let expected_json = r#"
          {
            "serialization_type": "borsh",
            "type_schema": {
              "declaration": "Tuple<u32, u32>",
              "definitions": {
                "Tuple<u32, u32>": {
                  "Tuple": ["u32", "u32"]
                }
              }
            }
          }
        "#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(value, expected_value);

        if let AbiType::Borsh { type_schema } = serde_json::from_str(expected_json).unwrap() {
            assert_eq!(type_schema.declaration, "Tuple<u32, u32>".to_string());
            assert_eq!(type_schema.definitions.len(), 1);
            assert_eq!(
                type_schema.definitions.get("Tuple<u32, u32>").unwrap(),
                &Definition::Tuple { elements: vec!["u32".to_string(), "u32".to_string()] }
            );
        } else {
            panic!("Unexpected serialization type")
        }
    }

    #[test]
    fn test_deser_enum() {
        #[derive(BorshSchema)]
        enum Either {
            _Left(u32),
            _Right(u32),
        }
        let abi_type = AbiType::Borsh { type_schema: <Either>::schema_container() };
        let value = serde_json::to_value(&abi_type).unwrap();
        let expected_json = r#"
          {
            "serialization_type": "borsh",
            "type_schema": {
              "declaration": "Either",
              "definitions": {
                "Either": {
                  "Enum": [
                    ["_Left", "Either_Left"],
                    ["_Right", "Either_Right"]
                  ]
                },
                "Either_Left": {
                  "Struct": ["u32"]
                },
                "Either_Right": {
                  "Struct": ["u32"]
                }
              }
            }
          }
        "#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(value, expected_value);

        if let AbiType::Borsh { type_schema } = serde_json::from_str(expected_json).unwrap() {
            assert_eq!(type_schema.declaration, "Either".to_string());
            assert_eq!(type_schema.definitions.len(), 3);
            assert_eq!(
                type_schema.definitions.get("Either").unwrap(),
                &Definition::Enum {
                    variants: vec![
                        ("_Left".to_string(), "Either_Left".to_string()),
                        ("_Right".to_string(), "Either_Right".to_string())
                    ]
                }
            );
        } else {
            panic!("Unexpected serialization type")
        }
    }

    #[test]
    fn test_deser_struct_named() {
        #[derive(BorshSchema)]
        struct Pair {
            _first: u32,
            _second: u32,
        }
        let abi_type = AbiType::Borsh { type_schema: <Pair>::schema_container() };
        let value = serde_json::to_value(&abi_type).unwrap();
        let expected_json = r#"
          {
            "serialization_type": "borsh",
            "type_schema": {
              "declaration": "Pair",
              "definitions": {
                "Pair": {
                  "Struct": [
                    ["_first", "u32"],
                    ["_second", "u32"]
                  ]
                }
              }
            }
          }
        "#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(value, expected_value);

        if let AbiType::Borsh { type_schema } = serde_json::from_str(expected_json).unwrap() {
            assert_eq!(type_schema.declaration, "Pair".to_string());
            assert_eq!(type_schema.definitions.len(), 1);
            assert_eq!(
                type_schema.definitions.get("Pair").unwrap(),
                &Definition::Struct {
                    fields: Fields::NamedFields(vec![
                        ("_first".to_string(), "u32".to_string()),
                        ("_second".to_string(), "u32".to_string())
                    ])
                }
            );
        } else {
            panic!("Unexpected serialization type")
        }
    }

    #[test]
    fn test_deser_struct_unnamed() {
        #[derive(BorshSchema)]
        struct Pair(u32, u32);
        let abi_type = AbiType::Borsh { type_schema: <Pair>::schema_container() };
        let value = serde_json::to_value(&abi_type).unwrap();
        let expected_json = r#"
          {
            "serialization_type": "borsh",
            "type_schema": {
              "declaration": "Pair",
              "definitions": {
                "Pair": {
                  "Struct": [
                    "u32",
                    "u32"
                  ]
                }
              }
            }
          }
        "#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(value, expected_value);

        if let AbiType::Borsh { type_schema } = serde_json::from_str(expected_json).unwrap() {
            assert_eq!(type_schema.declaration, "Pair".to_string());
            assert_eq!(type_schema.definitions.len(), 1);
            assert_eq!(
                type_schema.definitions.get("Pair").unwrap(),
                &Definition::Struct {
                    fields: Fields::UnnamedFields(vec!["u32".to_string(), "u32".to_string()])
                }
            );
        } else {
            panic!("Unexpected serialization type")
        }
    }

    #[test]
    fn test_deser_struct_empty() {
        #[derive(BorshSchema)]
        struct Unit;
        let abi_type = AbiType::Borsh { type_schema: <Unit>::schema_container() };
        let value = serde_json::to_value(&abi_type).unwrap();
        let expected_json = r#"
          {
            "serialization_type": "borsh",
            "type_schema": {
              "declaration": "Unit",
              "definitions": {
                "Unit": {
                  "Struct": null
                }
              }
            }
          }
        "#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(value, expected_value);

        if let AbiType::Borsh { type_schema } = serde_json::from_str(expected_json).unwrap() {
            assert_eq!(type_schema.declaration, "Unit".to_string());
            assert_eq!(type_schema.definitions.len(), 1);
            assert_eq!(
                type_schema.definitions.get("Unit").unwrap(),
                &Definition::Struct { fields: Fields::Empty }
            );
        } else {
            panic!("Unexpected serialization type")
        }
    }
}
