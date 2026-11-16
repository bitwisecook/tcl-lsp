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

//! Joining names: normalising and qualifying a name under a namespace, rooting and unrooting constructed keys, member keys and colon trimming.

use super::constructed_keys::canonical_written_command;
use super::written_names::{
    ends_with_separator, is_absolute, is_absolute_bytes, qualifier_segments,
    qualifier_segments_owned,
};

/// Normalise a possibly-qualified Tcl command or procedure name.
///
/// Ensures the name starts with `::` and removes empty parts from
/// consecutive `::` separators.
///
/// ```
/// use tcl_syntax::naming::normalise_qualified_name;
/// assert_eq!(normalise_qualified_name("foo"), "::foo");
/// assert_eq!(normalise_qualified_name("ns::bar"), "::ns::bar");
/// assert_eq!(normalise_qualified_name("::baz"), "::baz");
/// assert_eq!(normalise_qualified_name(""), "");
/// assert_eq!(normalise_qualified_name("::::x"), "::x");
/// ```
#[must_use]
pub fn normalise_qualified_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // Share the one canonical `::` segmentation. Each segment is a subslice of a
    // `&str` split on ASCII `::`, so it is valid UTF-8.
    let parts: Vec<&str> = qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| core::str::from_utf8(s).expect("subslice of valid UTF-8"))
        .collect();
    if parts.is_empty() {
        return "::".to_owned();
    }
    format!("::{}", parts.join("::"))
}

/// Join a namespace `prefix` and a (possibly-relative) `name` into a fully
/// qualified, `::`-rooted name — the canonical Tcl qualification rule:
///
/// * An **absolute** `name` (leading `::`) ignores the prefix entirely —
///   `qualify("::ns", "::other::C")` is `::other::C`, never re-prefixed.
/// * A relative `name` resolves under `prefix`, which may be given rooted
///   (`::a::b`) or unrooted (`a::b`).  The *written* `name`'s colon runs
///   collapse ([`canonical_written_command`]); the `prefix` is a constructed
///   key and is joined verbatim.
/// * An empty / root prefix roots the name at `::`.
///
/// The one shared join for the analyser / signature-scan / class-lattice
/// qualifiers, so the absolute-name rule cannot drift between them.
#[must_use]
pub fn qualify(prefix: &str, name: &str) -> String {
    // Canonicalise the *written* name once (colon-run rule, trailing-separator
    // preservation), then join with one exact separator — never re-parse the
    // joined result, which would collapse a legitimately colon-named segment
    // into its parent (`proc :` inside `namespace eval :`).
    let canonical = canonical_written_command(name);
    if is_absolute(name) {
        return canonical;
    }
    // The prefix is a *constructed* namespace key (possibly unrooted); use it
    // verbatim apart from rooting.  (Stripping is single-shot: the key of a
    // namespace named `:` is `":::"`, whose unrooted form is `":"`.)
    let p = prefix.strip_prefix("::").unwrap_or(prefix);
    if p.is_empty() {
        if canonical.is_empty() {
            // The empty-name entity (`proc {} {} {}`) at the root — see
            // [`ends_with_separator`]; `::x::` is its form inside `::x`.
            return "::".to_owned();
        }
        return format!("::{canonical}");
    }
    format!("::{p}::{canonical}")
}

/// Qualify a written C namespace name under a constructed namespace key.
/// Trailing separators select the namespace itself; empty relative input
/// retains the current namespace. This is an authored naming abstraction,
/// without native namespace existence or execution authority.
#[must_use]
pub fn qualify_namespace(prefix: &str, written: &str) -> String {
    if is_absolute(written) {
        return normalise_qualified_name(written);
    }
    let parts = qualifier_segments_owned(written);
    if parts.is_empty() {
        let relative = unroot_rooted_key(prefix).unwrap_or(prefix);
        return if relative.is_empty() {
            "::".to_owned()
        } else {
            format!("::{relative}")
        };
    }
    qualify(prefix, &parts.join("::"))
}

/// Byte-preserving qualification for command glob patterns and names.
/// The prefix is constructed; only the written name's colon runs collapse.
#[must_use]
pub fn qualify_bytes(prefix: &[u8], name: &[u8]) -> Vec<u8> {
    let mut result = if is_absolute_bytes(name) {
        b"::".to_vec()
    } else {
        let prefix = prefix.strip_prefix(b"::").unwrap_or(prefix);
        let mut rooted = b"::".to_vec();
        rooted.extend_from_slice(prefix);
        if !prefix.is_empty() {
            rooted.extend_from_slice(b"::");
        }
        rooted
    };
    for (index, segment) in qualifier_segments(name).into_iter().enumerate() {
        if index != 0 {
            result.extend_from_slice(b"::");
        }
        result.extend_from_slice(segment);
    }
    if ends_with_separator(name) && !name.iter().all(|byte| *byte == b':') {
        result.extend_from_slice(b"::");
    }
    result
}

/// Add the global root marker to an **unrooted constructed key**.
///
/// This is not written-name canonicalisation: the input has already been
/// constructed from namespace segments, so every byte must be retained.  In
/// particular, an unrooted key may itself begin with `::` when its first
/// segment contains literal colons; prepend exactly one root marker rather
/// than using [`canonical_written_command`] or testing whether it already
/// looks rooted.
///
/// ```
/// use tcl_syntax::naming::root_unrooted_key;
///
/// assert_eq!(root_unrooted_key(""), "::");
/// assert_eq!(root_unrooted_key("matrix"), "::matrix");
/// assert_eq!(root_unrooted_key(":::p"), ":::::p");
/// ```
#[must_use]
pub fn root_unrooted_key(key: &str) -> String {
    format!("::{key}")
}

/// Counted-byte twin of [`root_unrooted_key`]: every byte of the constructed
/// key is retained after exactly one prepended root marker.
#[must_use]
pub fn root_unrooted_key_bytes(key: &[u8]) -> Vec<u8> {
    let mut rooted = Vec::with_capacity(key.len() + 2);
    rooted.extend_from_slice(b"::");
    rooted.extend_from_slice(key);
    rooted
}

/// String twin of [`rooted_member_key_bytes`]: neither key is reparsed.
///
/// ```
/// use tcl_syntax::naming::rooted_member_key;
/// assert_eq!(rooted_member_key("::", "v"), "::v");
/// assert_eq!(rooted_member_key("::a", "b"), "::a::b");
/// assert_eq!(rooted_member_key(":::", ":"), "::::::");
/// ```
#[must_use]
pub fn rooted_member_key(rooted_namespace: &str, member: &str) -> String {
    if rooted_namespace == "::" {
        format!("::{member}")
    } else {
        format!("{rooted_namespace}::{member}")
    }
}

/// A member's rooted reporting key under a rooted constructed namespace key:
/// the root's members take one marker, any other namespace one separator.
/// Neither input is reparsed; this supplies no lookup or table identity.
///
/// ```
/// use tcl_syntax::naming::rooted_member_key_bytes;
/// assert_eq!(rooted_member_key_bytes(b"::", b"v"), b"::v");
/// assert_eq!(rooted_member_key_bytes(b"::a:", b":v"), b"::a::::v");
/// ```
#[must_use]
pub fn rooted_member_key_bytes(rooted_namespace: &[u8], member: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(rooted_namespace.len() + member.len() + 2);
    key.extend_from_slice(rooted_namespace);
    if rooted_namespace != b"::" {
        key.extend_from_slice(b"::");
    }
    key.extend_from_slice(member);
    key
}

/// A member's key under an **unrooted constructed namespace key**, the
/// counterpart of [`rooted_member_key`] for tables keyed without the root
/// marker. The root is the empty key there, so its members stand alone; any
/// other namespace takes one separator. Neither input is reparsed.
///
/// ```
/// use tcl_syntax::naming::unrooted_member_key;
/// assert_eq!(unrooted_member_key("", "v"), "v");
/// assert_eq!(unrooted_member_key("a", "b"), "a::b");
/// assert_eq!(unrooted_member_key("a::b", ""), "a::b::");
/// assert_eq!(unrooted_member_key(":", ":"), "::::");
/// ```
#[must_use]
pub fn unrooted_member_key(unrooted_namespace: &str, member: &str) -> String {
    if unrooted_namespace.is_empty() {
        member.to_owned()
    } else {
        format!("{unrooted_namespace}::{member}")
    }
}

/// Remove exactly the root marker from a **rooted constructed key**.
/// This is the inverse of [`root_unrooted_key`], preserving colon-named
/// segments rather than canonicalising the spelling as a written name.
///
/// ```
/// use tcl_syntax::naming::{root_unrooted_key, unroot_rooted_key};
/// for key in ["", "N", ":", ":::p"] {
///     assert_eq!(unroot_rooted_key(&root_unrooted_key(key)), Some(key));
/// }
/// assert_eq!(unroot_rooted_key("relative"), None);
/// ```
#[must_use]
pub fn unroot_rooted_key(key: &str) -> Option<&str> {
    key.strip_prefix("::")
}

/// Remove one root marker from a counted constructed key, retaining all other bytes.
#[must_use]
pub fn unroot_rooted_key_bytes(key: &[u8]) -> Option<&[u8]> {
    key.strip_prefix(b"::")
}

/// A written name without its leading colons, for a display label or a
/// registry spelling: `::a::b` is `a::b`, `:::a` is `a`. It drops a single
/// leading colon too, so it is not a lookup key; a lookup uses
/// [`unroot_rooted_key`] or [`canonical_written_command`].
///
/// ```
/// use tcl_syntax::naming::without_leading_colons;
/// assert_eq!(without_leading_colons("::switch"), "switch");
/// assert_eq!(without_leading_colons(":::a::b"), "a::b");
/// assert_eq!(without_leading_colons("plain"), "plain");
/// ```
#[must_use]
pub fn without_leading_colons(name: &str) -> &str {
    name.trim_start_matches(':')
}

/// A literal fragment of a partly dynamic name without the colons at either
/// end, which belong to the separators joining it to its neighbours:
/// `::ns::` is `ns`, `a::b` stays `a::b`.
///
/// ```
/// use tcl_syntax::naming::without_surrounding_colons;
/// assert_eq!(without_surrounding_colons("::ns::"), "ns");
/// assert_eq!(without_surrounding_colons("a::b"), "a::b");
/// assert_eq!(without_surrounding_colons("::"), "");
/// ```
#[must_use]
pub fn without_surrounding_colons(fragment: &str) -> &str {
    fragment.trim_matches(':')
}

/// The analyser's flat key for a variable held by a named scope: the scope's
/// own key, one separator, the variable's base name. The scope key is used
/// verbatim, so this is a pair rendered flat and not a Tcl name.
///
/// ```
/// use tcl_syntax::naming::scope_variable_key;
/// assert_eq!(scope_variable_key("::", "x"), "::::x");
/// assert_eq!(scope_variable_key("::ns::p", "x"), "::ns::p::x");
/// ```
#[must_use]
pub fn scope_variable_key(scope: &str, variable: &str) -> String {
    format!("{scope}::{variable}")
}
