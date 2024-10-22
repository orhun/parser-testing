#![allow(dead_code)]
#![allow(clippy::should_implement_trait)]

use std::fmt::{self, Display};

use serde::{
    de::{self, Error as SerdeError, Visitor},
    forward_to_deserialize_any,
};

/// Representation of all possible data types that may exist.
pub enum Data {
    Pair((String, String)),
    List((String, Vec<String>)),
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
    input: Vec<Data>,
}

impl Deserializer {
    // Create a new deserializer from a string
    pub fn from_str(input: &str) -> Self {
        let input = vec![
            Data::Pair(("mykey".to_string(), "some_value".to_string())),
            Data::List((
                "mykeylist".to_string(),
                vec![
                    "some_list_value".to_string(),
                    "some_other_list_value".to_string(),
                ],
            )),
        ];
        Deserializer { input }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer {
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        true
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_map(MapAccessTop(self))
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_seq(SeqAccessTop(self))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_seq(SeqAccessTop(self))
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_some(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf unit unit_struct newtype_struct tuple tuple_struct
        struct identifier ignored_any enum
    }
}
