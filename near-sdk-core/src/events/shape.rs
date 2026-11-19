use serde::ser::{Impossible, SerializeMap, SerializeStruct, SerializeStructVariant};
use serde::{Serialize, Serializer};
use std::fmt;

pub(super) fn has_event_key<T: Serialize>(value: &T) -> bool {
    value.serialize(ShapeSerializer).unwrap_or(false)
}

#[derive(Debug)]
struct ShapeError;

impl fmt::Display for ShapeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("payload has no top-level fields")
    }
}

impl std::error::Error for ShapeError {}

impl serde::ser::Error for ShapeError {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        Self
    }
}

struct ShapeSerializer;
struct Fields(bool);

macro_rules! scalar {
    ($($method:ident($type:ty)),* $(,)?) => {$(
        fn $method(self, _value: $type) -> Result<bool, ShapeError> { Ok(false) }
    )*};
}

impl Serializer for ShapeSerializer {
    type Ok = bool;
    type Error = ShapeError;
    type SerializeSeq = Impossible<bool, ShapeError>;
    type SerializeTuple = Impossible<bool, ShapeError>;
    type SerializeTupleStruct = Impossible<bool, ShapeError>;
    type SerializeTupleVariant = Impossible<bool, ShapeError>;
    type SerializeMap = Fields;
    type SerializeStruct = Fields;
    type SerializeStructVariant = Fields;

    scalar!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8]),
    );

    fn serialize_none(self) -> Result<bool, ShapeError> {
        Ok(false)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<bool, ShapeError> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<bool, ShapeError> {
        Ok(false)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<bool, ShapeError> {
        Ok(false)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
    ) -> Result<bool, ShapeError> {
        Ok(false)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<bool, ShapeError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<bool, ShapeError> {
        Ok(false)
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, ShapeError> {
        Err(ShapeError)
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, ShapeError> {
        Err(ShapeError)
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, ShapeError> {
        Err(ShapeError)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, ShapeError> {
        Err(ShapeError)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Fields, ShapeError> {
        Ok(Fields(false))
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Fields, ShapeError> {
        Ok(Fields(false))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Fields, ShapeError> {
        Ok(Fields(false))
    }
    fn collect_str<T: ?Sized + fmt::Display>(self, _value: &T) -> Result<bool, ShapeError> {
        Ok(false)
    }
}

impl SerializeMap for Fields {
    type Ok = bool;
    type Error = ShapeError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), ShapeError> {
        self.0 |= matches!(serde_json::to_string(key).as_deref(), Ok(r#""event""#));
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<(), ShapeError> {
        Ok(())
    }

    fn end(self) -> Result<bool, ShapeError> {
        Ok(self.0)
    }
}

macro_rules! fields {
    ($trait:ident) => {
        impl $trait for Fields {
            type Ok = bool;
            type Error = ShapeError;
            fn serialize_field<T: ?Sized + Serialize>(
                &mut self,
                key: &'static str,
                _value: &T,
            ) -> Result<(), ShapeError> {
                self.0 |= key == "event";
                Ok(())
            }
            fn end(self) -> Result<bool, ShapeError> {
                Ok(self.0)
            }
        }
    };
}

fields!(SerializeStruct);
fields!(SerializeStructVariant);

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::SerializeMap;
    use std::collections::BTreeMap;

    #[derive(Serialize)]
    struct EventStruct {
        event: &'static str,
        data: u64,
    }

    #[derive(Serialize)]
    struct PlainStruct {
        amount: u64,
    }

    #[derive(Serialize)]
    struct UnitStruct;

    #[derive(Serialize)]
    struct Newtype(u64);

    #[derive(Serialize)]
    struct TupleStruct(u64, u64);

    #[derive(Serialize)]
    enum Variants {
        Unit,
        Newtype(u64),
        Tuple(u64, u64),
        Struct { amount: u64 },
    }

    struct Bytes;

    impl Serialize for Bytes {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(b"event")
        }
    }

    struct Collected;

    impl Serialize for Collected {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(&"event")
        }
    }

    struct EntryMap;

    impl Serialize for EntryMap {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("data", &5u64)?;
            map.serialize_entry("event", "minted")?;
            map.end()
        }
    }

    #[test]
    fn detects_event_in_structs_and_maps_regardless_of_order() {
        assert!(has_event_key(&EventStruct { event: "minted", data: 5 }));
        assert!(has_event_key(&EntryMap));
        assert!(!has_event_key(&PlainStruct { amount: 5 }));

        let mut map = BTreeMap::new();
        map.insert("amount", 5u64);
        assert!(!has_event_key(&map));
    }

    #[test]
    fn non_map_shapes_are_plain() {
        assert!(!has_event_key(&false));
        assert!(!has_event_key(&-1i8));
        assert!(!has_event_key(&-1i16));
        assert!(!has_event_key(&-1i32));
        assert!(!has_event_key(&-1i64));
        assert!(!has_event_key(&-1i128));
        assert!(!has_event_key(&1u8));
        assert!(!has_event_key(&1u16));
        assert!(!has_event_key(&1u32));
        assert!(!has_event_key(&1u64));
        assert!(!has_event_key(&1u128));
        assert!(!has_event_key(&1.0f32));
        assert!(!has_event_key(&1.0f64));
        assert!(!has_event_key(&'e'));
        assert!(!has_event_key(&"event"));
        assert!(!has_event_key(&Bytes));
        assert!(!has_event_key(&Option::<u64>::None));
        assert!(!has_event_key(&Some(5u64)));
        assert!(!has_event_key(&()));
        assert!(!has_event_key(&UnitStruct));
        assert!(!has_event_key(&Newtype(5)));
        assert!(!has_event_key(&vec![1u64, 2]));
        assert!(!has_event_key(&(1u64, 2u64)));
        assert!(!has_event_key(&TupleStruct(1, 2)));
        assert!(!has_event_key(&Variants::Unit));
        assert!(!has_event_key(&Variants::Newtype(5)));
        assert!(!has_event_key(&Variants::Tuple(1, 2)));
        assert!(!has_event_key(&Variants::Struct { amount: 5 }));
        assert!(!has_event_key(&Collected));
    }
}
