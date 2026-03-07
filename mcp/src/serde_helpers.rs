//! Shared serde deserialization helpers
//!
//! MCP clients may inconsistently send numeric parameters as JSON numbers or
//! strings. These helpers accept both formats for robust deserialization.

use std::fmt;
use std::str::FromStr;

use serde::Deserializer;
use serde::de;
use serde::de::Visitor;

/// Deserialize a numeric value from either a JSON number or a string.
///
/// Accepts both `42` and `"42"` for compatibility with MCP clients that may
/// send numeric parameters as strings.
pub(super) fn deserialize_number_or_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: TryFrom<u64> + FromStr + fmt::Display,
    <T as TryFrom<u64>>::Error: fmt::Display,
    <T as FromStr>::Err: fmt::Display,
{
    struct NumberOrStringVisitor<T>(std::marker::PhantomData<T>);

    impl<T> Visitor<'_> for NumberOrStringVisitor<T>
    where
        T: TryFrom<u64> + FromStr + fmt::Display,
        <T as TryFrom<u64>>::Error: fmt::Display,
        <T as FromStr>::Err: fmt::Display,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number or numeric string")
        }

        fn visit_u64<E>(self, value: u64) -> Result<T, E>
        where
            E: de::Error,
        {
            T::try_from(value).map_err(|e| E::custom(e))
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            value.parse::<T>().map_err(|e| E::custom(e))
        }
    }

    deserializer.deserialize_any(NumberOrStringVisitor(std::marker::PhantomData))
}
