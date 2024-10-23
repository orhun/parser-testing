#![allow(dead_code)]
#![allow(clippy::should_implement_trait)]

use core::num;
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    marker::PhantomData,
    str::FromStr,
};

use serde::{
    de::{self, DeserializeOwned, IntoDeserializer, Visitor},
    forward_to_deserialize_any, Deserialize,
};

/// ------------------ Crate entry points ------------------

/// Deserialize an instance of type `T` from a string of INI text.
pub fn from_str<T: DeserializeOwned>(s: &str) -> DeResult<T> {
    let mut de = Deserializer::from_str(s);
    let value = Deserialize::deserialize(&mut de)?;

    Ok(value)
}

/// ------------------ High-level dataformat deserialization logic ------------------

#[derive(Debug, Clone)]
pub enum Error {
    /// Custom Error, neccessary to interface with serde.
    Custom(String),

    /// Deserialization error
    /// Passed through error message from the parser.
    ParserError(String),

    /// Internal consistency error
    InvalidState,
}

pub type DeResult<T> = std::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Custom(msg) => write!(f, "Serde error: {}", msg),
            Error::ParserError(msg) => write!(f, "Error occured during parsing::{}", msg),
            Error::InvalidState => write!(f, "internal consistency error"),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "ALPM file deserialization error"
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

// Necessary to handle from_str parser errors during deserialization
impl From<num::ParseIntError> for Error {
    fn from(e: num::ParseIntError) -> Self {
        Error::Custom(e.to_string())
    }
}

impl From<num::ParseFloatError> for Error {
    fn from(e: num::ParseFloatError) -> Self {
        Error::Custom(e.to_string())
    }
}

/// ------------------ Deserialization initialization ------------------

pub struct Deserializer {
    input: BTreeMap<String, Data>,
}

impl Deserializer {
    // Create a new deserializer from a string.
    // The string will be parsed and put into a intermediate representation in the form of
    // `BTreeMap<String, Data>`
    pub fn from_str(_input: &str) -> Self {
        let mut input = BTreeMap::new();
        input.insert("key".to_string(), Data::Value("value".to_string()));
        input.insert(
            "list".to_string(),
            Data::List(vec!["1".to_string(), "2".to_string()]),
        );
        input.insert(
            "number_list".to_string(),
            Data::List(vec!["1".to_string(), "2".to_string()]),
        );

        input.insert("single_key_list".to_string(), Data::Value("yo".to_string()));
        input.insert("u64".to_string(), Data::Value("1".to_string()));
        input.insert("u32".to_string(), Data::Value("10".to_string()));
        input.insert("i64".to_string(), Data::Value("-1".to_string()));
        input.insert("i32".to_string(), Data::Value("-10".to_string()));
        Deserializer { input }
    }
}

/// ------------------ High-level dataformat deserialization logic ------------------

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        true
    }

    fn deserialize_any<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(self.input.clone().into_deserializer())
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf unit unit_struct newtype_struct tuple_struct
        struct identifier ignored_any enum option map tuple seq
    }
}

/// ------------------ Data Deserialization ------------------

/// Representation of raw parsed data.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Data {
    Value(String),
    List(Vec<String>),
}

impl Data {
    pub fn value_or_error(&self) -> Result<&str, Error> {
        match self {
            Data::Value(value) => Ok(value),
            Data::List(_) => Err(Error::InvalidState),
        }
    }
}

impl<'de> IntoDeserializer<'de, Error> for Data {
    type Deserializer = DataDeserializer<Error>;

    fn into_deserializer(self) -> Self::Deserializer {
        DataDeserializer::new(self)
    }
}

pub struct DataDeserializer<E> {
    data: Data,
    marker: PhantomData<E>,
}

impl<E> DataDeserializer<E> {
    pub fn new(data: Data) -> Self {
        DataDeserializer {
            data,
            marker: PhantomData,
        }
    }
}

impl<'de> de::Deserializer<'de> for DataDeserializer<Error> {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        true
    }

    fn deserialize_any<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match &self.data {
            Data::Value(value) => visitor.visit_str(value),
            Data::List(vec) => visitor.visit_seq(vec.clone().into_deserializer()),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> DeResult<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match &self.data {
            Data::Value(value) => visitor.visit_seq(vec![value.clone()].into_deserializer()),
            Data::List(vec) => visitor.visit_seq(vec.clone().into_deserializer()),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        self.deserialize_any(visitor)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_i8(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_i16(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_i32(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_i64(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_i128(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_u8(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_u16(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_u32(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_u64(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_u128(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_f32(FromStr::from_str(self.data.value_or_error()?)?)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> DeResult<V::Value> {
        visitor.visit_f64(FromStr::from_str(self.data.value_or_error()?)?)
    }

    forward_to_deserialize_any! {
        char str string bytes
        byte_buf unit unit_struct newtype_struct tuple tuple_struct
        struct identifier ignored_any enum option map
    }
}
