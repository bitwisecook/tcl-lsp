//! `RouteDomain` value type — the F5 `%N` route-domain suffix.

use super::error::ValueError;
use super::port::{parse_decimal_int, py_repr};
use std::fmt;

/// An F5 route-domain identifier (the `%N` suffix on addresses).
///
/// Route-domain 0 is the default (no segregation); higher numbers select
/// specific routing tables on multi-tenant deployments. Stored as the
/// integer id; rendered as `%N` so it concatenates cleanly behind an
/// address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteDomain {
    /// The numeric route-domain id (>= 0).
    pub id: u32,
}

impl RouteDomain {
    /// Construct directly from a numeric id.
    #[must_use]
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    /// Parse `text` as a route-domain id.
    ///
    /// Accepts `"5"`, `"%5"`, and `"   5  "`.
    ///
    /// # Errors
    /// Negatives, non-integers, and empty input return [`ValueError`].
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let mut text = text.trim();
        if let Some(rest) = text.strip_prefix('%') {
            text = rest;
        }
        if text.is_empty() {
            return Err(ValueError("RouteDomain: empty input".to_owned()));
        }
        let value = parse_decimal_int(text)
            .ok_or_else(|| ValueError(format!("RouteDomain: not numeric ({})", py_repr(text))))?;
        if value < 0 {
            return Err(ValueError(format!("RouteDomain: negative ({value})")));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self { id: value as u32 })
    }

    /// Like [`RouteDomain::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// `true` when this is route-domain 0 (no segregation).
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.id == 0
    }
}

impl fmt::Display for RouteDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variants() {
        assert_eq!(RouteDomain::parse("5").unwrap().id, 5);
        assert_eq!(RouteDomain::parse("%5").unwrap().id, 5);
        assert_eq!(RouteDomain::parse("   5  ").unwrap().id, 5);
        assert_eq!(RouteDomain::parse("0").unwrap().id, 0);
        assert!(RouteDomain::parse("0").unwrap().is_default());
    }

    #[test]
    fn round_trips() {
        assert_eq!(RouteDomain::parse("%10").unwrap().to_string(), "%10");
        assert_eq!(RouteDomain::new(0).to_string(), "%0");
    }

    #[test]
    fn rejects_bad() {
        assert_eq!(
            RouteDomain::parse("%").unwrap_err().0,
            "RouteDomain: empty input"
        );
        assert_eq!(
            RouteDomain::parse("-3").unwrap_err().0,
            "RouteDomain: negative (-3)"
        );
        assert_eq!(
            RouteDomain::parse("abc").unwrap_err().0,
            "RouteDomain: not numeric ('abc')"
        );
        assert!(RouteDomain::try_parse("xx").is_none());
    }
}
