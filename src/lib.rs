#![allow(dead_code)]
#![allow(clippy::should_implement_trait)]

use std::{
    collections::BTreeMap,
    fmt::{self, Display},
};

use serde::{
    de::{
        self, value::MapDeserializer, DeserializeOwned, Error as SerdeError, IntoDeserializer,
        Visitor,
    },
    forward_to_deserialize_any, Deserialize,
};

/// Deserialize an instance of type `T` from a string of INI text.
pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T> {
    let mut de = Deserializer::from_str(s);
    let value = Deserialize::deserialize(&mut de)?;

    Ok(value)
}

/// Representation of all possible data types that may exist.
#[derive(Debug, Deserialize)]
pub enum Data {
    Value(String),
    List(Vec<String>),
}

impl<'de> IntoDeserializer<'de> for Data {
    type Deserializer = T;

    fn into_deserializer(self) -> Self::Deserializer {
        match self {
            Data::Value(value) => value.into_deserializer(),
            Data::List(vec) => vec.into_deserializer(),
        }
    }
}

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

pub type Result<T> = std::result::Result<T, Error>;

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

impl SerdeError for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

pub struct Deserializer {
    input: BTreeMap<String, Data>,
}

impl Deserializer {
    // Create a new deserializer from a string
    pub fn from_str(input: &str) -> Self {
        let mut input = BTreeMap::new();
        input.insert("mykey".to_string(), Data::Value("some_value".to_string()));
        input.insert(
            "mykeylist".to_string(),
            Data::List(vec![
                "some_list_value".to_string(),
                "some_other_list_value".to_string(),
            ]),
        );
        Deserializer { input }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        true
    }

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(MapDeserializer::new(self.input.into_iter()))
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor
            .visit_map(MapDeserializer::new(self.input.into_iter()))
            .map_err()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf unit unit_struct newtype_struct tuple tuple_struct
        struct identifier ignored_any enum option seq
    }
}
