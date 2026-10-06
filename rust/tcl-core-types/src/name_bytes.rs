// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact name storage and constructed namespace paths.
//!
//! These types preserve bytes. Native operation policies select input extents
//! before a runtime stores a key; this module performs no such selection.

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{borrow::Borrow, ops::Deref, str::Utf8Error};

/// Immutable byte identity without Unicode or written-name interpretation.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NameBytes(Arc<[u8]>);

impl NameBytes {
    /// Exact retained bytes, including NUL and non-Unicode native units.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// A checked analytical view. Failure does not make the name invalid.
    ///
    /// # Errors
    /// Returns an error when a retained native component is not valid Rust UTF-8.
    pub fn try_utf8(&self) -> Result<&str, Utf8Error> {
        core::str::from_utf8(&self.0)
    }

    /// Transfer the shared storage without decoding it.
    #[must_use]
    pub fn into_arc(self) -> Arc<[u8]> {
        self.0
    }
}

impl core::fmt::Debug for NameBytes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("NameBytes").field(&self.as_bytes()).finish()
    }
}
impl Borrow<[u8]> for NameBytes {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl AsRef<[u8]> for NameBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl Deref for NameBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl From<Arc<[u8]>> for NameBytes {
    fn from(bytes: Arc<[u8]>) -> Self {
        Self(bytes)
    }
}
impl From<Vec<u8>> for NameBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes.into())
    }
}
impl From<&[u8]> for NameBytes {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.into())
    }
}
impl<const N: usize> From<&[u8; N]> for NameBytes {
    fn from(bytes: &[u8; N]) -> Self {
        Self::from(bytes.as_slice())
    }
}
impl From<&str> for NameBytes {
    fn from(text: &str) -> Self {
        Self::from(text.as_bytes())
    }
}
impl From<String> for NameBytes {
    fn from(text: String) -> Self {
        Self::from(text.into_bytes())
    }
}

/// Already constructed namespace segments. Literal colons remain segment data.
///
/// A written spelling must be parsed by its selected naming policy before
/// constructing this path. A display spelling cannot recover every such path.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteNamespacePath(Vec<NameBytes>);

impl ByteNamespacePath {
    /// The root namespace has no segments.
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    /// Construct a path from components that already have their boundaries.
    #[must_use]
    pub fn from_segments(segments: impl IntoIterator<Item = impl Into<NameBytes>>) -> Self {
        Self(segments.into_iter().map(Into::into).collect())
    }

    /// Exact constructed components, without joining or reparsing them.
    #[must_use]
    pub fn as_segments(&self) -> &[NameBytes] {
        &self.0
    }

    /// Whether this is the root namespace.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Add one already selected child component.
    pub fn push(&mut self, segment: impl Into<NameBytes>) {
        self.0.push(segment.into());
    }

    /// Remove one child component without interpreting its bytes.
    pub fn pop(&mut self) -> Option<NameBytes> {
        self.0.pop()
    }

    /// Parent path, or no parent for the root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let mut parent = self.clone();
        parent.pop()?;
        Some(parent)
    }

    /// Child path retaining the current constructed components.
    #[must_use]
    pub fn with_child(&self, segment: impl Into<NameBytes>) -> Self {
        let mut child = self.clone();
        child.push(segment);
        child
    }

    /// Transfer the exact constructed components.
    #[must_use]
    pub fn into_segments(self) -> Vec<NameBytes> {
        self.0
    }
}
impl AsRef<[NameBytes]> for ByteNamespacePath {
    fn as_ref(&self) -> &[NameBytes] {
        self.as_segments()
    }
}
impl Deref for ByteNamespacePath {
    type Target = [NameBytes];
    fn deref(&self) -> &[NameBytes] {
        self.as_segments()
    }
}
impl From<Vec<NameBytes>> for ByteNamespacePath {
    fn from(segments: Vec<NameBytes>) -> Self {
        Self(segments)
    }
}

/// Structured byte location, independent of namespace arena identity.
pub type ByteCommandSlot = crate::CommandSlot<NameBytes, ByteNamespacePath>;
/// Byte location carried through native compilation and bridge snapshots.
pub type NativeByteCommandSlot = ByteCommandSlot;

impl PartialEq<str> for NameBytes {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl PartialEq<&str> for NameBytes {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl PartialEq<String> for NameBytes {
    fn eq(&self, other: &String) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl From<&String> for NameBytes {
    fn from(text: &String) -> Self {
        Self::from(text.as_bytes())
    }
}
impl<T: Into<NameBytes>> FromIterator<T> for ByteNamespacePath {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_segments(iter)
    }
}
impl<const N: usize> PartialEq<[&str; N]> for ByteNamespacePath {
    fn eq(&self, other: &[&str; N]) -> bool {
        self.0.len() == N
            && self
                .0
                .iter()
                .zip(other)
                .all(|(left, right)| left.as_bytes() == right.as_bytes())
    }
}
impl<const N: usize> PartialEq<[String; N]> for ByteNamespacePath {
    fn eq(&self, other: &[String; N]) -> bool {
        self.0.len() == N
            && self
                .0
                .iter()
                .zip(other)
                .all(|(left, right)| left.as_bytes() == right.as_bytes())
    }
}

/// Return the prefix before the first literal NUL byte.
///
/// This primitive performs no decoding or name-policy selection. Only a
/// purpose-specific protocol decides whether a native boundary uses this
/// extent; length-delimited object bytes remain complete until that decision.
#[must_use]
pub fn c_string_extent(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{collections::BTreeMap, vec};

    #[test]
    fn c_string_extent_selects_only_literal_nul() {
        assert_eq!(c_string_extent(b"k\0z"), b"k");
        assert_eq!(c_string_extent(b"\0z"), b"");
        assert_eq!(c_string_extent(b"k\xc0\x80z"), b"k\xc0\x80z");
        assert_eq!(c_string_extent(b"k\xff"), b"k\xff");
        assert_eq!(c_string_extent(b""), b"");
    }

    #[test]
    fn native_byte_keys_never_collapse_encoding_or_nul_controls() {
        let keys = [
            b"k\0z".as_slice(),
            b"k",
            b"k\xc0\x80z",
            b"k\xff",
            b"k\xc3\xbf",
            b"k\xed\xa0\x80",
            b"k\xef\xbf\xbd",
        ];
        let map: BTreeMap<NameBytes, usize> = keys
            .iter()
            .enumerate()
            .map(|(index, bytes)| (NameBytes::from(*bytes), index))
            .collect();
        assert_eq!(map.len(), keys.len());
        for (index, key) in keys.into_iter().enumerate() {
            assert_eq!(map.get(key), Some(&index));
        }
        assert!(NameBytes::from(b"k\0z").try_utf8().is_ok());
        assert!(NameBytes::from(b"k\xc0\x80z").try_utf8().is_err());
        assert!(NameBytes::from(b"k\xff").try_utf8().is_err());
    }

    #[test]
    fn constructed_paths_do_not_reparse_literal_colons_or_display_collisions() {
        let first = ByteCommandSlot::new(
            ByteNamespacePath::from_segments([b"a:".as_slice()]),
            NameBytes::from(b"p"),
        );
        let second = ByteCommandSlot::new(
            ByteNamespacePath::from_segments([b"a".as_slice()]),
            NameBytes::from(b":p"),
        );
        assert_ne!(first, second);
        assert_eq!(first.namespace.as_segments(), &[NameBytes::from(b"a:")]);
        let child = first.namespace.with_child(b"\xff\0:");
        assert_eq!(child.parent(), Some(first.namespace));
        assert_eq!(
            child.into_segments(),
            vec![NameBytes::from(b"a:"), NameBytes::from(b"\xff\0:")]
        );
        assert_eq!(NameBytes::from("p"), "p");
        assert_eq!(ByteNamespacePath::from_segments(["a", "b"]), ["a", "b"]);
    }
}
