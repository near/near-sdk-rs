use near_sdk::borsh::BorshSchema;
use near_sdk::NearSchema;
use near_sdk::__private::schemars::JsonSchema;
use std::marker::PhantomData;

// https://stackoverflow.com/a/71721454/9806233
// https://github.com/nvzqz/impls/blob/e616c2d65615aa04cd266dd9f7bcab14e2a10d50/src/lib.rs#L647-L661
macro_rules! impls {
    ($ty:ty: $trait:ident) => {{
        trait DoesNotImpl {
            const IMPLS: bool = false;
        }
        impl<T: ?Sized> DoesNotImpl for T {}

        struct Wrapper<T: ?Sized>(PhantomData<T>);

        #[allow(dead_code)]
        impl<T: ?Sized + $trait> Wrapper<T> {
            const IMPLS: bool = true;
        }

        <Wrapper<$ty>>::IMPLS
    }};
}

macro_rules! const_assert_impls {
    ($ty:ty: $trait:ident) => {
        const _: () = {
            assert!(
                impls!($ty: $trait),
                concat!(stringify!($ty), " does not implement ", stringify!($trait))
            );
        };
    };
    ($ty:ty: !$trait:ident) => {
        const _: () = {
            assert!(
                !impls!($ty: $trait),
                concat!(stringify!($ty), " implements ", stringify!($trait), " but shouldn't")
            );
        };
    };
}

pub fn non_mod_scoped() {
    #[derive(NearSchema)]
    struct InnerValue;

    const_assert_impls!(InnerValue: JsonSchema);
    const_assert_impls!(InnerValue: !BorshSchema);

    #[derive(NearSchema)]
    struct Value {
        field: InnerValue,
    }

    const_assert_impls!(Value: JsonSchema);
    const_assert_impls!(Value: !BorshSchema);
}

pub fn no_schema_spec() {
    #[derive(NearSchema)]
    #[serde(rename = "UnitNoSchemaSpecSTRUCT")]
    struct UnitStructNoSchemaSpec;

    const_assert_impls!(UnitStructNoSchemaSpec: JsonSchema);
    const_assert_impls!(UnitStructNoSchemaSpec: !BorshSchema);

    #[derive(NearSchema)]
    #[serde(rename = "UNITNoSchemaSpecENUM")]
    pub enum UnitEnumNoSchemaSpec {}

    const_assert_impls!(UnitEnumNoSchemaSpec: JsonSchema);
    const_assert_impls!(UnitEnumNoSchemaSpec: !BorshSchema);

    #[derive(NearSchema)]
    #[serde(rename = "NoSchemaSpecENUM")]
    pub enum EnumNoSchemaSpec {
        NoAttrs,
        #[serde(rename = "serde_via_schemars")]
        Serde,
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        Nested {
            #[serde(alias = "inner_inner_hehe")]
            nested: UnitEnumNoSchemaSpec,
        },
    }

    const_assert_impls!(EnumNoSchemaSpec: JsonSchema);
    const_assert_impls!(EnumNoSchemaSpec: !BorshSchema);

    #[derive(NearSchema)]
    #[serde(rename = "NoSchemaSpecSTRUCT")]
    struct StructNoSchemaSpec {
        var1: EnumNoSchemaSpec,
        var2: EnumNoSchemaSpec,
    }

    const_assert_impls!(StructNoSchemaSpec: JsonSchema);
    const_assert_impls!(StructNoSchemaSpec: !BorshSchema);
}

pub fn json_schema_spec() {
    #[derive(NearSchema)]
    #[abi(json)]
    #[serde(rename = "UnitNoSchemaSpecSTRUCT")]
    pub struct UnitStructNoSchemaSpec;

    const_assert_impls!(UnitStructNoSchemaSpec: JsonSchema);
    const_assert_impls!(UnitStructNoSchemaSpec: !BorshSchema);

    #[derive(NearSchema)]
    #[abi(json)]
    #[serde(rename = "UNITNoSchemaSpecENUM")]
    pub enum UnitEnumNoSchemaSpec {}

    const_assert_impls!(UnitEnumNoSchemaSpec: JsonSchema);
    const_assert_impls!(UnitEnumNoSchemaSpec: !BorshSchema);

    #[derive(NearSchema)]
    #[abi(json)]
    #[serde(rename = "NoSchemaSpecENUM")]
    pub enum EnumNoSchemaSpec {
        NoAttrs,
        #[serde(rename = "serde_via_schemars")]
        Serde,
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        Nested {
            #[serde(alias = "inner_inner_hehe")]
            nested: UnitEnumNoSchemaSpec,
        },
    }

    const_assert_impls!(EnumNoSchemaSpec: JsonSchema);
    const_assert_impls!(EnumNoSchemaSpec: !BorshSchema);

    #[derive(NearSchema)]
    #[abi(json)]
    #[serde(rename = "NoSchemaSpecSTRUCT")]
    struct StructNoSchemaSpec {
        var1: EnumNoSchemaSpec,
        var2: EnumNoSchemaSpec,
    }

    const_assert_impls!(StructNoSchemaSpec: JsonSchema);
    const_assert_impls!(StructNoSchemaSpec: !BorshSchema);
}

pub fn borsh_schema_spec() {
    #[derive(NearSchema)]
    #[abi(borsh)]
    pub struct UnitStructNoSchemaSpec;

    const_assert_impls!(UnitStructNoSchemaSpec: BorshSchema);
    const_assert_impls!(UnitStructNoSchemaSpec: !JsonSchema);

    #[derive(NearSchema)]
    #[abi(borsh)]
    pub enum UnitEnumNoSchemaSpec {}

    const_assert_impls!(UnitEnumNoSchemaSpec: BorshSchema);
    const_assert_impls!(UnitEnumNoSchemaSpec: !JsonSchema);

    #[derive(NearSchema)]
    #[abi(borsh)]
    pub enum EnumNoSchemaSpec {
        NoAttrs,
        #[borsh_skip]
        BorshSkip,
        Nested {
            #[borsh_skip]
            // fixme! rust complains of an unread field here
            // fixme! https://github.com/near/borsh-rs/issues/111
            nested: UnitEnumNoSchemaSpec,
        },
    }

    const_assert_impls!(EnumNoSchemaSpec: BorshSchema);
    const_assert_impls!(EnumNoSchemaSpec: !JsonSchema);

    #[derive(NearSchema)]
    #[abi(borsh)]
    struct StructNoSchemaSpec {
        var1: EnumNoSchemaSpec,
        #[borsh_skip]
        var2: EnumNoSchemaSpec,
    }

    const_assert_impls!(StructNoSchemaSpec: BorshSchema);
    const_assert_impls!(StructNoSchemaSpec: !JsonSchema);
}

pub fn json_borsh_schema_spec() {
    #[derive(NearSchema)]
    #[abi(json, borsh)]
    #[serde(rename = "UnitNoSchemaSpecSTRUCT")]
    pub struct UnitStructNoSchemaSpec;

    const_assert_impls!(UnitStructNoSchemaSpec: JsonSchema);
    const_assert_impls!(UnitStructNoSchemaSpec: BorshSchema);

    #[derive(NearSchema)]
    #[abi(json, borsh)]
    #[serde(rename = "UNITNoSchemaSpecENUM")]
    pub enum UnitEnumNoSchemaSpec {}

    const_assert_impls!(UnitEnumNoSchemaSpec: JsonSchema);
    const_assert_impls!(UnitEnumNoSchemaSpec: BorshSchema);

    #[derive(NearSchema)]
    #[abi(json, borsh)]
    #[serde(rename = "NoSchemaSpecENUM")]
    pub enum EnumNoSchemaSpec {
        NoAttrs,
        #[borsh_skip]
        BorshSkip,
        #[serde(rename = "serde_via_schemars")]
        Serde,
        #[borsh_skip]
        #[serde(skip)]
        BorshSerde,
        #[serde(skip)]
        #[borsh_skip]
        SerdeBorsh,
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        Nested {
            #[borsh_skip]
            // fixme! borsh doesn't play well with nested attributes
            // fixme! https://github.com/near/borsh-rs/issues/110
            // #[serde(alias = "inner_inner_hehe")]
            nested: UnitEnumNoSchemaSpec,
        },
    }

    const_assert_impls!(EnumNoSchemaSpec: JsonSchema);
    const_assert_impls!(EnumNoSchemaSpec: BorshSchema);

    #[derive(NearSchema)]
    #[abi(json, borsh)]
    #[serde(rename = "NoSchemaSpecSTRUCT")]
    struct StructNoSchemaSpec {
        var1: EnumNoSchemaSpec,
        #[borsh_skip]
        var2: EnumNoSchemaSpec,
    }

    const_assert_impls!(StructNoSchemaSpec: JsonSchema);
    const_assert_impls!(StructNoSchemaSpec: BorshSchema);
}

fn main() {}
