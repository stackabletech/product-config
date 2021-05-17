//! This module provides a serde [`serde::ser::Serializer`] to convert a (more or less)
//! arbitrary struct into a [`HashMap`].
//!
//! This can be used in products using this library to provide a strongly typed struct with all configuration parameters which can then be converted into a HashMap as required by this library.
//!
//! # Example
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use product_config::ser;
//!
//! #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
//! #[serde(rename_all = "camelCase")]
//! pub struct TestConfig {
//!     pub option_one: Option<u32>,
//!     pub option_two: Option<String>
//! }
//!
//! let config = TestConfig {
//!   option_one: Some(123),
//!   option_two: None
//! };
//!
//! let config_map = ser::to_hash_map(&config).unwrap();
//!
//! ```
use serde::de;
use serde::ser::{self, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Message(String),
    UnsupportedType,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => write!(f, "{}", msg),
            Error::UnsupportedType => f.write_str("unsupported type"),
        }
    }
}

impl std::error::Error for Error {}

/// This method tries to convert any struct into a HashMap.
/// Other types (e.g. tuples, sequences etc.) are not supported
///
/// NOTE: There will be edge-cases that this method does not support.
/// One example being conflicts. Two things can map to the same key.
/// We don't currently check for that.
///
/// Field names of structs will be the keys of the resulting map.
/// These field types are supported:
///
/// * bool (conversion will be to "true" or "false")
/// * integer types
/// * floating-point types
/// * char
/// * String/str
/// * Option: Will be serialized as just the contained value if it's Some. None will be omitted entirely.
/// * Unit: Will be omitted
/// * Unit struct: Will be omitted
/// * Enum
/// * Newtype structs: Will be serialized as the data they contain (the "wrapper" will be ignored)
/// * Newtype variant (Newtype variant of enums): Will be serialized as the data they contain (that means the Enum variant name will be ignored as well as the newtype wrapper!)
/// * Map: The fields of the nested map will be emitted using a dotted syntax (e.g. "parent_field.nested_field")
/// * structs: See Map
/// * struct variant: See Map
///
/// These are supported with some limitations:
/// * sequences (e.g. Vec)
/// * tuple
/// * tuple struct
/// * tuple variant (see sequence)
///
/// The limitation being that currently we do not support any of these in a nested fashion (e.g. a vector of tuples).
/// There will be no error but the result will be undefined.
/// This is an implementation limitation that can be lifted later if needed.
///
/// These are not supported:
/// * bytes
pub fn to_hash_map<T>(value: &T) -> Result<HashMap<String, String>>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        output: HashMap::new(),
        current_field: None,
        sequence: None,
        value: None,
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

/// The Serializer is the struct that implements the serde::ser::Serializer trait.
/// It is used to collect intermediate data while we walk the source object.
// TODO: We need to detect when we're being called on something that is not a Map, Struct or Struct Variant
struct Serializer {
    output: HashMap<String, String>,

    // This stores the current field name which includes all its parents.
    // The parents will be concatenated using dots (".", e.g. "foo.bar")
    current_field: Option<String>,

    // Here we're collecting a sequence of values before we can move it to the `value` field
    // TODO: Nested sequences will break this. It'll require a better design.
    sequence: Option<String>,

    // Due to the way serde works we need a way to also store the intermediate results of each field
    // after conversion to a String
    value: Option<String>,
}

impl<'a> ser::Serializer for &'a mut Serializer {
    // This is the output type of the Serializer.
    // According to its docs most Serializers should set this to `()` and output to a buffer instead.
    // That's exactly what we're doing.
    // We use the Serializer::output map as our buffer.
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    // Not sure what to make out of a byte array.
    // Could be converted into a String but for now we don't support it.
    fn serialize_bytes(self, _: &[u8]) -> Result<()> {
        Err(Error::UnsupportedType)
    }

    fn serialize_bool(self, v: bool) -> Result<()> {
        let value = if v {
            "true".to_string()
        } else {
            "false".to_string()
        };

        self.value = Some(value);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    // The serde docs say this about this approach:
    // "Not particularly efficient.
    // A more performant approach would be to use the `itoa` crate."
    //
    // Performance doesn't really matter much for this piece of code which is why we
    // are using this naive approach.
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.value = Some(v.to_string());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.value = Some(v.to_string());
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.value = Some(v.to_string());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.value = Some(v.to_string());
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.value = Some(v.to_string());
        Ok(())
    }

    fn serialize_unit(self) -> Result<()> {
        self.value = None;
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.value = None;
        Ok(())
    }

    // A present optional is represented as just the contained value.
    // This is potentially a lossy representation if the contained value also serializes
    // to a "null" value but for our use-case it's probably the correct choice.
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    // Unit struct means a named value containing no data.
    // Again, since there is no data, this will be omitted entirely.
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    // Will be serialized as the value only
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // Note that newtype variant (and all of the other variant serialization
    // methods) refer exclusively to the "externally tagged" enum
    // representation.
    //
    // Serialize this to JSON in externally tagged form as `{ NAME: VALUE }`.
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.sequence = None;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    // Unit variants are enum variants without any value.
    // In this case we'll serialize the name of the variant.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    // An enum variant where the data is a tuple
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(self)
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, _: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::UnsupportedType)
    }

    fn serialize_value<T>(&mut self, _: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::UnsupportedType)
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: Serialize,
        V: Serialize,
    {
        key.serialize(&mut **self)?;
        let key = self.value.take();

        value.serialize(&mut **self)?;
        let value = self.value.take();

        if let (Some(key), Some(value)) = (key, value) {
            self.output.insert(
                format!("{}.{}", self.current_field.as_ref().unwrap(), key),
                value,
            );
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // If we are already "within" another object we'll append a dot and our current field name
        // to this name.
        let original_field = self.current_field.clone();
        if let Some(parent_key) = &self.current_field {
            self.current_field = Some(format!("{}.{}", parent_key, key))
        } else {
            self.current_field = Some(key.to_string());
        }

        value.serialize(&mut **self)?;
        let value = self.value.take();
        if let Some(value) = value {
            self.output
                .insert(self.current_field.as_ref().unwrap().to_string(), value);
        }

        self.current_field = original_field;

        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)?;
        if let Some(ref value) = self.value {
            // If our sequence already contains Some we need to append a comma (TODO: Make configurable)
            // At this point we're certain that the current value serializes to something
            if let Some(current_sequence) = self.sequence.as_mut() {
                current_sequence.push_str(",");
            }

            self.sequence
                .get_or_insert_with(String::new)
                .push_str(value);
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        self.value = self.sequence.take();
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        if let Some(ref value) = self.value {
            // If our sequence already contains Some we need to append a comma (TODO: Make configurable)
            // At this point we're certain that the current value serializes to something
            if let Some(current_sequence) = self.sequence.as_mut() {
                current_sequence.push_str(",");
            }

            self.sequence
                .get_or_insert_with(String::new)
                .push_str(value);
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.value = self.sequence.take();
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        if let Some(ref value) = self.value {
            // If our sequence already contains Some we need to append a comma (TODO: Make configurable)
            // At this point we're certain that the current value serializes to something
            if let Some(current_sequence) = self.sequence.as_mut() {
                current_sequence.push_str(",");
            }

            self.sequence
                .get_or_insert_with(String::new)
                .push_str(value);
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.value = self.sequence.take();
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        if let Some(ref value) = self.value {
            // If our sequence already contains Some we need to append a comma (TODO: Make configurable)
            // At this point we're certain that the current value serializes to something
            if let Some(current_sequence) = self.sequence.as_mut() {
                current_sequence.push_str(",");
            }

            self.sequence
                .get_or_insert_with(String::new)
                .push_str(value);
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.value = self.sequence.take();
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // If we are already "within" another object we'll append a dot and our current field name
        // to this name.
        let original_field = self.current_field.clone();
        if let Some(parent_key) = &self.current_field {
            self.current_field = Some(format!("{}.{}", parent_key, key))
        } else {
            self.current_field = Some(key.to_string());
        }

        value.serialize(&mut **self)?;
        let value = self.value.take();
        if let Some(value) = value {
            self.output
                .insert(self.current_field.as_ref().unwrap().to_string(), value);
        }

        self.current_field = original_field;

        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::to_hash_map;
    use serde::Serialize;
    use std::collections::HashMap;

    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct NewtypeStruct(String);

        #[derive(Serialize)]
        struct UnitStruct;

        #[derive(Serialize)]
        struct TupleStruct(i16, u8);

        #[derive(Serialize)]
        struct TestStruct {
            nested_value: i32,
            nested_string: String,
        }

        #[derive(Serialize)]
        enum TestEnum {
            Unit,
            Newtype(String),
            Tuple(u32, u32),
            Struct { a: u16, b: u32 },
        }

        let mut test_map = HashMap::new();
        test_map.insert("foo".to_string(), 123);
        test_map.insert("bar".to_string(), 456);

        // TODO: Doesn't work: nested_sequence: Vec<(i16, u8)>,
        //  This fails: nested_sequence: vec![(1, 2), (3, 4)],
        #[derive(Serialize)]
        struct Test {
            bool_test: bool,

            i8_test: i8,
            i16_test: i16,
            i32_test: i32,
            i64_test: i64,

            u8_test: u8,
            u16_test: u16,
            u32_test: u32,
            u64_test: u64,

            f32_test: f32,
            f64_test: f64,

            char_test: char,
            string_test: String,
            unit_test: (),

            opt_none_test: Option<String>,
            opt_some_test: Option<String>,

            map_test: HashMap<String, i32>,

            enum_unit_variant_test: TestEnum,
            enum_newtype_variant_test: TestEnum,
            enum_tuple_variant_test: TestEnum,
            enum_struct_variant_test: TestEnum,

            sequence_test: Vec<String>,
            tuple_test: (String, i8),

            newtype_struct_test: NewtypeStruct,
            struct_test: TestStruct,
            tuple_struct_test: TupleStruct,
            unit_struct_test: UnitStruct,
        }

        let test = Test {
            bool_test: false,

            i8_test: -8,
            i16_test: -16,
            i32_test: -32,
            i64_test: -64,

            u8_test: 8,
            u16_test: 16,
            u32_test: 32,
            u64_test: 64,

            f32_test: 32.32,
            f64_test: 64.64,

            char_test: 'l',
            string_test: "test_string".to_string(),
            unit_test: (),

            opt_none_test: None,
            opt_some_test: Some("test_opt_str".to_string()),

            map_test: test_map,

            enum_unit_variant_test: TestEnum::Unit,
            enum_newtype_variant_test: TestEnum::Newtype("foobar".to_string()),
            enum_tuple_variant_test: TestEnum::Tuple(111, 222),
            enum_struct_variant_test: TestEnum::Struct { a: 1, b: 2 },

            sequence_test: vec!["one".to_string(), "two".to_string(), "three".to_string()],
            tuple_test: ("first_tuple_thing".to_string(), 123),

            newtype_struct_test: NewtypeStruct("foobar".to_string()),
            struct_test: TestStruct {
                nested_value: 1234,
                nested_string: "nested".to_string(),
            },
            tuple_struct_test: TupleStruct(1, 2),
            unit_struct_test: UnitStruct,
        };

        let mut map = to_hash_map(&test).unwrap();

        assert_eq!(map.remove("bool_test").unwrap(), "false");

        assert_eq!(map.remove("i8_test").unwrap(), "-8");
        assert_eq!(map.remove("i16_test").unwrap(), "-16");
        assert_eq!(map.remove("i32_test").unwrap(), "-32");
        assert_eq!(map.remove("i64_test").unwrap(), "-64");

        assert_eq!(map.remove("u8_test").unwrap(), "8");
        assert_eq!(map.remove("u16_test").unwrap(), "16");
        assert_eq!(map.remove("u32_test").unwrap(), "32");
        assert_eq!(map.remove("u64_test").unwrap(), "64");

        assert!(map.remove("f32_test").unwrap().starts_with("32"));
        assert!(map.remove("f64_test").unwrap().starts_with("64"));

        assert_eq!(map.remove("char_test").unwrap(), "l");
        assert_eq!(map.remove("string_test").unwrap(), "test_string");

        assert_eq!(map.remove("opt_some_test").unwrap(), "test_opt_str");

        assert_eq!(map.remove("map_test.foo").unwrap(), "123");
        assert_eq!(map.remove("map_test.bar").unwrap(), "456");

        assert_eq!(map.remove("enum_unit_variant_test").unwrap(), "Unit");
        assert_eq!(map.remove("enum_newtype_variant_test").unwrap(), "foobar");
        assert_eq!(map.remove("enum_tuple_variant_test").unwrap(), "111,222");
        assert_eq!(map.remove("enum_struct_variant_test.a").unwrap(), "1");
        assert_eq!(map.remove("enum_struct_variant_test.b").unwrap(), "2");

        assert_eq!(map.remove("sequence_test").unwrap(), "one,two,three");
        assert_eq!(map.remove("tuple_test").unwrap(), "first_tuple_thing,123");

        assert_eq!(map.remove("newtype_struct_test").unwrap(), "foobar");
        assert_eq!(map.remove("struct_test.nested_value").unwrap(), "1234");
        assert_eq!(map.remove("struct_test.nested_string").unwrap(), "nested");
        assert_eq!(map.remove("tuple_struct_test").unwrap(), "1,2");

        assert!(map.is_empty());
    }
}
