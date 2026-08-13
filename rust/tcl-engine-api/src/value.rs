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

//! The interface's value model: owned, structured, engine-neutral.
//!
//! A host and an engine exchange these, never strings that each side re-parses.
//! `list`/`dict` are first-class because the calling conventions above this
//! layer are list- and dict-shaped (`words` is a list, `ctx` is a dict), and
//! flattening them to text at the boundary is exactly the round-trip
//! `docs/design/spec-packs.md` rules out.

use std::rc::Rc;

/// One value crossing the host/engine boundary.
///
/// Cheap to clone: the compound variants share their storage. An engine
/// converts to and from its own representation at the boundary, and is free to
/// keep the parts (`Rc<str>`) rather than copying them.
#[derive(Debug, Clone)]
pub enum Value {
    /// The empty string — Tcl's ubiquitous "nothing".
    Empty,
    /// A string.
    Str(Rc<str>),
    /// An integer. Distinct from [`Self::Str`] so an engine with a native
    /// integer representation is not forced to re-parse one.
    Int(i64),
    /// A float.
    Double(f64),
    /// A Tcl list.
    List(Rc<[Value]>),
    /// A Tcl dict, in declaration order (Tcl dicts preserve insertion order).
    Dict(Rc<[(Value, Value)]>),
}

impl Value {
    /// A string value.
    #[must_use]
    pub fn string(text: impl AsRef<str>) -> Self {
        let text = text.as_ref();
        if text.is_empty() {
            Self::Empty
        } else {
            Self::Str(Rc::from(text))
        }
    }

    /// A list value.
    #[must_use]
    pub fn list(items: impl IntoIterator<Item = Self>) -> Self {
        Self::List(items.into_iter().collect())
    }

    /// A dict value.
    #[must_use]
    pub fn dict(entries: impl IntoIterator<Item = (Self, Self)>) -> Self {
        Self::Dict(entries.into_iter().collect())
    }

    /// A dict from string keys.
    #[must_use]
    pub fn dict_of<K: AsRef<str>>(entries: impl IntoIterator<Item = (K, Self)>) -> Self {
        Self::dict(
            entries
                .into_iter()
                .map(|(key, value)| (Self::string(key), value)),
        )
    }

    /// This value as a borrowed string, when it is one.
    ///
    /// Deliberately partial: a host that wants "the Tcl string form of
    /// anything" is asking the *engine* for a conversion, which is the
    /// engine's semantics (list quoting, float formatting), not this type's.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Empty => Some(""),
            Self::Str(text) => Some(text),
            Self::Int(_) | Self::Double(_) | Self::List(_) | Self::Dict(_) => None,
        }
    }

    /// This value as an integer, when it is one.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// This value's list elements, when it is a list.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// This value's dict entries, when it is a dict.
    #[must_use]
    pub fn as_dict(&self) -> Option<&[(Self, Self)]> {
        match self {
            Self::Dict(entries) => Some(entries),
            _ => None,
        }
    }

    /// Whether this is the empty string.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Self::string(text)
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Self::string(text)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Self::Int(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Int(i64::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn the_empty_string_is_one_representation() {
        assert!(Value::string("").is_empty());
        assert_eq!(Value::string("").as_str(), Some(""));
        assert_eq!(Value::string("x").as_str(), Some("x"));
    }

    #[test]
    fn compound_values_keep_their_structure() {
        let words = Value::list([Value::string("a"), Value::Int(2)]);
        assert_eq!(words.as_list().expect("a list").len(), 2);
        let ctx = Value::dict_of([("nwords", Value::Int(2))]);
        let entries = ctx.as_dict().expect("a dict");
        assert_eq!(entries[0].0.as_str(), Some("nwords"));
        assert_eq!(entries[0].1.as_int(), Some(2));
    }
}
