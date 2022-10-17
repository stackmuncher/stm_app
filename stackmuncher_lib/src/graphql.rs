//! This module is needed to support the cloud-side of the project.
//! It enables GraphQL support for core structures used on the client and server sides.

use juniper::{
    graphql_scalar,
    parser::{ParseError, ScalarToken},
    serde::{de, Deserialize, Deserializer, Serialize},
    InputValue, ParseScalarResult, ScalarValue, Value,
};
use std::{convert::TryInto as _, fmt};

/// An extension to the standard GraphQL set of types to include Rust scalar values.
/// Only the types used in this project are added to the list.
/// ### About GraphQL scalars
/// * https://graphql.org/learn/schema/#scalar-types
/// * https://www.graphql-tools.com/docs/scalars#custom-scalars
/// ### About extending the GraphQL scalars in Juniper
/// * https://graphql-rust.github.io/juniper/master/types/scalars.html#custom-scalars
/// * https://github.com/graphql-rust/juniper/issues/862
#[derive(Clone, Debug, PartialEq, ScalarValue, Serialize)]
#[serde(untagged)]
pub enum RustScalarValue {
    /// A GraphQL scalar for i32
    #[value(as_float, as_int)]
    Int(i32),
    /// A custom scalar for u64. The value is serialized into JSON number and should not be more than 53 bits to fit into JS Number type:
    /// * Number.MAX_SAFE_INTEGER = 2^53 - 1 = 9_007_199_254_740_991
    /// * https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number
    /// JSON spec does not constrain integer values unless specified in the schema. 53 bits is sufficient for our purposes.
    U64(u64),
    /// A custom scalar for i64 used in EPOCH timestamps. Theoretically, the value should never be negative because all STM dates are post 1970.
    /// The value is serialized into JSON number and should not be more than 53 bits to fit into JS Number type:
    /// * Number.MIN_SAFE_INTEGER = -(2^53 - 1) = -9,007,199,254,740,991
    I64(i64),
    /// A GraphQL scalar for f64
    #[value(as_float)]
    Float(f64),
    /// A GraphQL scalar for String
    #[value(as_str, as_string, into_string)]
    String(String),
    /// A GraphQL scalar for bool
    #[value(as_bool)]
    Boolean(bool),
}

impl<'de> Deserialize<'de> for RustScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = RustScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid input value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(RustScalarValue::Boolean(b))
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                Ok(RustScalarValue::Int(n))
            }

            fn visit_u64<E: de::Error>(self, b: u64) -> Result<Self::Value, E> {
                if b <= u64::from(i32::MAX as u32) {
                    self.visit_i32(b.try_into().unwrap())
                } else {
                    Ok(RustScalarValue::U64(b))
                }
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
                if n <= i32::MAX as u32 {
                    self.visit_i32(n.try_into().unwrap())
                } else {
                    self.visit_u64(n.into())
                }
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
                if n <= i64::MAX as i64 {
                    self.visit_i64(n.try_into().unwrap())
                } else {
                    // Browser's `JSON.stringify()` serializes all numbers
                    // having no fractional part as integers (no decimal point),
                    // so we must parse large integers as floating point,
                    // otherwise we would error on transferring large floating
                    // point numbers.
                    // TODO: Use `FloatToInt` conversion once stabilized:
                    //       https://github.com/rust-lang/rust/issues/67057
                    Ok(RustScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(RustScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                self.visit_string(s.into())
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(RustScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}

#[graphql_scalar(with = u64_scalar, scalar = RustScalarValue)]
pub type U64 = u64;

pub mod u64_scalar {
    use super::*;

    pub(super) fn to_output(v: &U64) -> Value<RustScalarValue> {
        Value::scalar(*v)
    }

    pub(super) fn from_input(v: &InputValue<RustScalarValue>) -> Result<U64, String> {
        v.as_scalar_value::<u64>()
            .copied()
            .ok_or_else(|| format!("Expected `RustScalarValue::U64`, found: {}", v))
    }

    pub(super) fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<RustScalarValue> {
        match value {
            ScalarToken::Int(v) => v
                .parse()
                .map_err(|_| ParseError::UnexpectedToken(v.into()))
                .map(|s: u64| s.into()),
            ScalarToken::Float(v) | ScalarToken::String(v) => Err(ParseError::UnexpectedToken(v.into())),
        }
    }
}

#[graphql_scalar(with = i64_scalar, scalar = RustScalarValue)]
pub type I64 = i64;

pub mod i64_scalar {
    use super::*;

    pub(super) fn to_output(v: &I64) -> Value<RustScalarValue> {
        Value::scalar(*v)
    }

    pub(super) fn from_input(v: &InputValue<RustScalarValue>) -> Result<I64, String> {
        v.as_scalar_value::<i64>()
            .copied()
            .ok_or_else(|| format!("Expected `RustScalarValue::I64`, found: {}", v))
    }

    pub(super) fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<RustScalarValue> {
        match value {
            ScalarToken::Int(v) => v
                .parse()
                .map_err(|_| ParseError::UnexpectedToken(v.into()))
                .map(|s: i64| s.into()),
            ScalarToken::Float(v) | ScalarToken::String(v) => Err(ParseError::UnexpectedToken(v.into())),
        }
    }
}
