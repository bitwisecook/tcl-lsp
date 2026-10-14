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

//! Constructed-key geometry fixtures for the shared Syntax naming owner.

/// Split a namespace string into its `::`-delimited parts,
/// skipping empty segments. `"::"` yields `[]`, `"::foo::bar"`
/// yields `["foo", "bar"]`. This is the canonical
/// [`crate::naming::qualifier_segments_owned`] split (colon runs are a
/// single separator).
#[must_use]
fn namespace_parts(namespace: &str) -> Vec<String> {
    crate::naming::qualifier_segments_owned(namespace)
}

/// Extract the enclosing-namespace portion of a fully qualified
/// proc name.
///
/// - `"::"` → `"::"`.
/// - `"::foo"` → `"::"`.
/// - `"::a::b::c"` → `"::a::b"`.
#[must_use]
fn namespace_from_qualified(qname: &str) -> String {
    let parts = tcl_syntax::naming::written_command_parts(qname.as_bytes());
    let path = tcl_core_types::ByteNamespacePath::from_segments(parts.qualifiers);
    tcl_syntax::naming::checked_namespace_path_key(&path).expect("UTF-8 constructed key")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_parts_splits_and_filters_empty() {
        assert_eq!(namespace_parts("::"), Vec::<String>::new());
        assert_eq!(namespace_parts("::foo"), vec!["foo".to_string()]);
        assert_eq!(
            namespace_parts("::a::b::c"),
            vec!["a".to_string(), "b".into(), "c".into()],
        );
        // Colon runs collapse to one separator (tclsh8.6: `a:::b` names
        // `::a::b`) — no stray `:`-bearing segments.
        assert_eq!(
            namespace_parts("::a:::b::::c"),
            vec!["a".to_string(), "b".into(), "c".into()],
        );
    }

    #[test]
    fn namespace_from_qualified_peels_last_segment() {
        assert_eq!(namespace_from_qualified("::"), "::");
        assert_eq!(namespace_from_qualified("::foo"), "::");
        assert_eq!(namespace_from_qualified("::a::b"), "::a");
        assert_eq!(namespace_from_qualified("::a::b::c"), "::a::b");
        assert_eq!(namespace_from_qualified("::a::b::"), "::a::b");
        assert_eq!(namespace_from_qualified("a::b::::"), "::a::b");
    }
}
