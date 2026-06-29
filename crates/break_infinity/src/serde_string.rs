//! Serde helpers that (de)serialize a [`Decimal`] as a JSON **string**, matching
//! break_infinity.js's `toString`/`fromString` rather than the struct's default
//! `{ "m": .., "e": .. }` field representation.
//!
//! This is what Antimatter Dimensions saves use: every `Decimal` is stored as a
//! plain JSON string (`"1000"`, `"1e+100"`, `"5e-8"`, `"Infinity"`). The save
//! layer routes its DTO (Data Transfer Object) fields through these helpers; the 
//! type's derived serde impls are left untouched for our own internal serialization.
//!
//! Usage:
//! ```ignore
//! #[derive(Serialize, Deserialize)]
//! struct DTO {
//!     #[serde(with = "break_infinity::serde_string")]
//!     antimatter: Decimal,
//!     #[serde(with = "break_infinity::serde_string::option", default)]
//!     sacrificed: Option<Decimal>,
//! }
//! ```
//!
//! On serialize we emit [`Decimal`]'s [`Display`](core::fmt::Display) output; on
//! deserialize we parse with [`Decimal::from_str`]. Both branch on exponent the
//! same way the JS library does, so `"1e+100"`-style strings round-trip and the
//! original game's `new Decimal(str)` accepts what we write. Per the save design
//! we do not aim for byte-identical mantissa precision, only for strings that
//! parse back into an equal `Decimal`.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};

use crate::Decimal;

/// Serializes a [`Decimal`] as its `Display` string (e.g. `"1e+100"`).
pub fn serialize<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // `collect_str` formats via `Display` without an intermediate `String`.
    serializer.collect_str(value)
}

/// Deserializes a [`Decimal`] from a string via [`Decimal::from_str`].
pub fn deserialize<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(DecimalStrVisitor)
}

struct DecimalStrVisitor;

impl Visitor<'_> for DecimalStrVisitor {
    type Value = Decimal;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a break_infinity Decimal string, e.g. \"1000\" or \"1e+100\"")
    }

    fn visit_str<E>(self, value: &str) -> Result<Decimal, E>
    where
        E: de::Error,
    {
        Decimal::from_str(value).map_err(de::Error::custom)
    }
}

/// Helpers for `Option<Decimal>` fields stored as a string-or-null.
///
/// Use via `#[serde(with = "break_infinity::serde_string::option")]`. `None`
/// serializes as JSON `null`; pair with `#[serde(default)]` to also accept a
/// missing field.
pub mod option {
    use super::*;

    pub fn serialize<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(decimal) => serializer.collect_str(decimal),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionDecimalStrVisitor)
    }

    struct OptionDecimalStrVisitor;

    impl<'de> Visitor<'de> for OptionDecimalStrVisitor {
        type Value = Option<Decimal>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a break_infinity Decimal string or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            super::deserialize(deserializer).map(Some)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::Decimal;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        #[serde(with = "crate::serde_string")]
        value: Decimal,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct OptHolder {
        #[serde(with = "crate::serde_string::option", default)]
        value: Option<Decimal>,
    }

    fn holder(value: Decimal) -> Holder {
        Holder { value }
    }

    #[test]
    fn serializes_as_json_string() {
        let cases = [
            // Plain-number branch (exponent in (-7, 21)).
            (r#"{"value":"1000"}"#, Decimal::from_float(1000.0)),
            (r#"{"value":"0"}"#, Decimal::ZERO),
            // Scientific branch.
            (r#"{"value":"1.0000000000000000e+100"}"#, Decimal::new(1.0, 100)),
            // Sentinels
            (r#"{"value":"Infinity"}"#, Decimal::MAX_VALUE),
            (r#"{"value":"NaN"}"#, Decimal::NAN),
        ];
        for (expected, value) in cases {
            let json = serde_json::to_string(&holder(value)).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn deserializes_from_json_string() {
        // Both the plain and the JS-style exponential forms must parse.
        let cases = [
            (r#"{"value":"1000"}"#, Decimal::from_float(1000.0)),
            (r#"{"value":"1e+100"}"#, Decimal::new(1.0, 100)),
            (r#"{"value":"1.5e+100"}"#, Decimal::new(1.5, 100)),
            (r#"{"value":"5e-8"}"#, Decimal::new(5.0, -8)),
            (r#"{"value":"Infinity"}"#, Decimal::MAX_VALUE),
            (r#"{"value":"-Infinity"}"#, Decimal::MIN_VALUE),
        ];
        for (json, expected) in cases {
            let holder: Holder = serde_json::from_str(json).unwrap();
            assert_eq!(holder.value, expected, "parsing {json}");
        }
    }

    #[test]
    fn rejects_non_string_and_garbage() {
        // Numbers (the default `{m,e}`-style) are not accepted here.
        assert!(serde_json::from_str::<Holder>(r#"{"value":1000}"#).is_err());
        assert!(serde_json::from_str::<Holder>(r#"{"value":"not-a-number"}"#).is_err());
    }

    #[test]
    fn round_trips_modelled_values() {
        let values = [
            Decimal::ZERO,
            Decimal::ONE,
            Decimal::from_float(1e12),
            Decimal::from_float(1.79e3),
            Decimal::new(3.5, -8),
            Decimal::new(1.5, 308),
            Decimal::new(2.0, 9000),
            Decimal::MAX_VALUE,
        ];
        for value in values {
            let json = serde_json::to_string(&holder(value)).unwrap();
            let back: Holder = serde_json::from_str(&json).unwrap();
            assert_eq!(back.value, value, "round-trip of {value}");
        }
    }

    #[test]
    fn option_helper() {
        let cases = [
            (r#"{"value":null}"#, OptHolder { value: None }),
            (
                r#"{"value":"1.0000000000000000e+50"}"#, 
                OptHolder { value: Some(Decimal::new(1.0, 50))}
            ),

        ];
        for (expected, obj) in cases {
            let json = serde_json::to_string(&obj).unwrap();
            assert_eq!(json, expected);
        }

        // null, missing, and a present string all deserialize as expected.
        let cases = [
            (r#"{"value":null}"#, OptHolder { value: None }),
            (r#"{}"#, OptHolder { value: None }),
            (
                r#"{"value":"1e+50"}"#, 
                OptHolder { value: Some(Decimal::new(1.0, 50)) }
            ),
        ];
        for (json, expected) in cases {
            let obj = serde_json::from_str::<OptHolder>(json).unwrap();
            assert_eq!(obj, expected);
        }
    }
}
