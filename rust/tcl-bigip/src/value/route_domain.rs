// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `RouteDomain` value type — the F5 `%N` route-domain suffix.

use super::error::ValueError;
use super::port::{parse_decimal_int, py_repr};
use std::fmt;

use tcl_syntax::irules_address::{MAX_ROUTE_DOMAIN, RouteDomainError, split_route_domain};

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
        let id = u32::try_from(value)
            .ok()
            .filter(|&id| id <= MAX_ROUTE_DOMAIN)
            .ok_or_else(|| ValueError(format!("RouteDomain: out of range ({value})")))?;
        Ok(Self { id })
    }

    /// Like [`RouteDomain::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    pub(super) fn split_suffix(text: &str) -> Result<(&str, Option<u32>), ValueError> {
        split_route_domain(text).map_err(|error| match error {
            RouteDomainError::Invalid(digits) => {
                ValueError(format!("invalid route-domain suffix %{digits}"))
            }
            RouteDomainError::OutOfRange(digits) => {
                ValueError(format!("route domain out of range: {digits}"))
            }
        })
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
        assert!(RouteDomain::try_parse("65534").is_some());
        assert!(RouteDomain::try_parse("65535").is_none());
    }
}
