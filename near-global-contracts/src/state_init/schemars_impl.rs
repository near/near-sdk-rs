use std::collections::BTreeMap;

use crate::GlobalContractId;

use super::StateInitV1;

impl schemars::JsonSchema for StateInitV1 {
    fn schema_name() -> String {
        "StateInitV1".to_string()
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut properties = schemars::Map::new();
        properties.insert("code".to_string(), generator.subschema_for::<GlobalContractId>());
        properties
            .insert("data".to_string(), generator.subschema_for::<BTreeMap<String, String>>());

        schemars::schema::Schema::Object(schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::Object.into()),
            object: Some(Box::new(schemars::schema::ObjectValidation {
                properties,
                required: ["code".to_string(), "data".to_string()].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
