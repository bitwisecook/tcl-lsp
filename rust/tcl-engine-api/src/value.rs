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
//! `docs/design/registry/spec-packs.md` rules out.

use std::rc::Rc;

/// Physical resident-string storage, independent of its byte contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStringStorageIdentity {
    /// The selected native engine's canonical empty-string allocation.
    CanonicalEmpty,
    /// A distinct allocated string buffer, including allocated empty buffers.
    Allocated,
    /// Exact bytes are known but their native allocation identity is unavailable.
    Unknown,
}

/// C release belonging to an independently retained native cache descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCVersion {
    /// C Tcl 8.4.
    V8_4,
    /// C Tcl 8.5.
    V8_5,
    /// C Tcl 8.6.
    V8_6,
    /// C Tcl 9.0.
    V9_0,
    /// C Tcl 9.1.
    V9_1,
}

/// Radix of a full cached integer magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIntegerRadix {
    /// Binary magnitude digits.
    Binary,
    /// Octal magnitude digits.
    Octal,
    /// Decimal magnitude digits.
    Decimal,
    /// Hexadecimal magnitude digits.
    Hexadecimal,
}

/// Full primary scalar cache. This transports storage, not getter permission.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeScalarCache {
    /// C Tcl8.4's native long cache, distinct from its wide-integer descriptor.
    Tcl84Long(i64),
    /// Exact signed integer cache, independently of a narrower getter return.
    Integer(i64),
    /// Full integer magnitude, without wrapping or a floating-point conversion.
    BigInteger {
        /// Whether the magnitude is negative.
        negative: bool,
        /// Base of the cleaned magnitude digits.
        radix: NativeIntegerRadix,
        /// Magnitude digits without a sign, prefix or separators.
        digits: Rc<str>,
    },
    /// Exact floating-point payload, including NaN payload and signed zero.
    Double(f64),
    /// A parsed NaN cache with its original sign and optional mantissa payload.
    Nan {
        /// The parsed sign.
        negative: bool,
        /// The independently retained mantissa payload.
        payload: Option<u64>,
    },
    /// C's non-integer word-Boolean cache with its original descriptor release.
    WordBoolean {
        /// Boolean interpretation.
        value: bool,
        /// Actual release of the original native descriptor.
        origin: NativeCVersion,
    },
    /// Jim's exact integer retained under a coerced-double representation.
    JimCoercedInteger(i64),
}

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
    /// An exact resident native string, including non-Unicode bytes and NUL.
    StringBytes(Rc<[u8]>),
    /// Original binary storage whose string has not been materialised.
    ByteArray(Rc<[u8]>),
    /// An independently retained full native primary scalar cache.
    NativeScalar(NativeScalarCache),
    /// A typed payload with its already resident, authoritative string bytes.
    /// Neither the payload nor the spelling may be regenerated at the bridge.
    Resident {
        /// The independently retained typed payload.
        value: Rc<Self>,
        /// Exact existing native string bytes, excluding the C terminator.
        string: Rc<[u8]>,
        /// Actual allocation identity; empty bytes alone do not establish it.
        storage: NativeStringStorageIdentity,
    },
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
    /// Retain an exact native string without Unicode conversion or encoding.
    #[must_use]
    pub fn string_bytes(bytes: impl Into<Rc<[u8]>>) -> Self {
        Self::StringBytes(bytes.into())
    }

    /// Retain pure byte-array storage, independently of native string encoding.
    #[must_use]
    pub fn byte_array(bytes: impl Into<Rc<[u8]>>) -> Self {
        Self::ByteArray(bytes.into())
    }

    /// Attach an existing string to an independently known typed payload.
    /// This records actual storage; it does not perform native conversion.
    #[must_use]
    pub fn with_resident_string(self, string: impl Into<Rc<[u8]>>) -> Self {
        self.with_resident_string_storage(string, NativeStringStorageIdentity::Unknown)
    }

    /// Attach exact resident bytes and their independently known storage identity.
    /// The receiving engine validates that the payload and storage can coexist.
    #[must_use]
    pub fn with_resident_string_storage(
        self,
        string: impl Into<Rc<[u8]>>,
        storage: NativeStringStorageIdentity,
    ) -> Self {
        Self::Resident {
            value: Rc::new(self),
            string: string.into(),
            storage,
        }
    }

    /// Existing string bytes, without generating a string for a typed payload.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Empty => Some(b""),
            Self::Str(text) => Some(text.as_bytes()),
            Self::StringBytes(bytes) => Some(bytes),
            Self::Resident { string, .. } => Some(string),
            Self::ByteArray(_)
            | Self::NativeScalar(_)
            | Self::Int(_)
            | Self::Double(_)
            | Self::List(_)
            | Self::Dict(_) => None,
        }
    }

    /// Independently retained binary storage; no string-to-byte conversion.
    #[must_use]
    pub fn as_byte_array(&self) -> Option<&[u8]> {
        match self {
            Self::ByteArray(bytes) => Some(bytes),
            Self::Resident { value, .. } => value.as_byte_array(),
            _ => None,
        }
    }

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
        self.as_bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
    }

    /// This value as an integer, when it is one.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            Self::NativeScalar(
                NativeScalarCache::Integer(value) | NativeScalarCache::Tcl84Long(value),
            ) => Some(*value),
            Self::Resident { value, .. } => value.as_int(),
            _ => None,
        }
    }

    /// This value's list elements, when it is a list.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(items) => Some(items),
            Self::Resident { value, .. } => value.as_list(),
            _ => None,
        }
    }

    /// This value's dict entries, when it is a dict.
    #[must_use]
    pub fn as_dict(&self) -> Option<&[(Self, Self)]> {
        match self {
            Self::Dict(entries) => Some(entries),
            Self::Resident { value, .. } => value.as_dict(),
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
    #[test]
    fn resident_bytes_and_byte_array_backing_remain_independent() {
        let raw = Value::string_bytes(&b"a\0\xC0\x80\xFF"[..]);
        assert_eq!(raw.as_bytes(), Some(b"a\0\xC0\x80\xFF".as_slice()));
        assert_eq!(raw.as_str(), None);
        let pure = Value::byte_array(&b"\0\xFF"[..]);
        assert_eq!(pure.as_bytes(), None);
        let resident = pure.with_resident_string(&b"\xC0\x80\xC3\xBF"[..]);
        assert_eq!(resident.as_byte_array(), Some(b"\0\xFF".as_slice()));
        assert_eq!(resident.as_bytes(), Some(b"\xC0\x80\xC3\xBF".as_slice()));
        assert_eq!(resident.as_str(), None);
        let integer = Value::Int(16).with_resident_string(&b"0x10"[..]);
        assert_eq!(integer.as_int(), Some(16));
        assert_eq!(integer.as_bytes(), Some(b"0x10".as_slice()));
    }
}
