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

//! New namespace spellings for package scaffolds, independently of name lookup.

/// Construction policy shared with editor scaffolds. Different package labels
/// may produce the same spelling; callers own collision checks before creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaffoldNamespacePolicy {
    /// Literal separator in package display text.
    pub separator: &'static str,
    /// Root prefix added to the generated namespace.
    pub root: &'static str,
    /// Segment used when no nonempty input segment remains.
    pub fallback: &'static str,
    /// Prefix for a segment whose first unit is not an identifier start.
    pub identifier_prefix: &'static str,
    /// ASCII replacement for each unsupported UTF-16 input unit.
    pub replacement: char,
    /// Inclusive ASCII ranges accepted at a segment's first position.
    pub first_unit_ranges: &'static [(u16, u16)],
    /// Inclusive ASCII ranges accepted in a segment.
    pub following_unit_ranges: &'static [(u16, u16)],
}

/// Package scaffold names use ASCII identifiers and preserve each unsupported
/// UTF-16 unit as an underscore, including both units of a surrogate pair.
pub const PACKAGE_SCAFFOLD_NAMESPACE_POLICY: ScaffoldNamespacePolicy = ScaffoldNamespacePolicy {
    separator: "::",
    root: "::",
    fallback: "pkg",
    identifier_prefix: "_",
    replacement: '_',
    first_unit_ranges: &[(65, 90), (95, 95), (97, 122)],
    following_unit_ranges: &[(48, 57), (65, 90), (95, 95), (97, 122)],
};

/// A proposed source spelling, never an existing namespace identity or receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedScaffoldNamespace {
    /// Rooted ASCII spelling suitable for a new scaffold.
    pub spelling: String,
    /// Input units or empty segments were replaced or discarded.
    pub lossy: bool,
}

/// Construct a new package scaffold namespace. This does not resolve names or
/// establish uniqueness, existence, interpreter ownership or creation rights.
#[must_use]
pub fn scaffold_namespace_from_package_name(package_name: &str) -> GeneratedScaffoldNamespace {
    let policy = &PACKAGE_SCAFFOLD_NAMESPACE_POLICY;
    let accepts =
        |ranges: &[(u16, u16)], unit| ranges.iter().any(|&(lo, hi)| lo <= unit && unit <= hi);
    let mut parts = Vec::new();
    let mut lossy = false;
    for part in package_name.split(policy.separator) {
        if part.is_empty() {
            lossy = true;
            continue;
        }
        let mut cleaned = String::new();
        for unit in part.encode_utf16() {
            let character = if accepts(policy.following_unit_ranges, unit) {
                u8::try_from(unit).map_or(policy.replacement, char::from)
            } else {
                policy.replacement
            };
            cleaned.push(character);
        }
        if !cleaned
            .encode_utf16()
            .next()
            .is_some_and(|unit| accepts(policy.first_unit_ranges, unit))
        {
            cleaned.insert_str(0, policy.identifier_prefix);
        }
        lossy |= cleaned != part;
        parts.push(cleaned);
    }
    if parts.is_empty() {
        parts.push(policy.fallback.to_owned());
        lossy = true;
    }
    GeneratedScaffoldNamespace {
        spelling: format!("{}{}", policy.root, parts.join(policy.separator)),
        lossy,
    }
}

/// Cross-language construction cases emitted beside the editor's shared policy.
pub const PACKAGE_SCAFFOLD_NAMESPACE_CASES: &[&str] = &[
    "acme::http-utils",
    "",
    "::",
    "::::",
    "acme:::tools",
    "acme::::tools",
    "9tools",
    "café::tools",
    "💡::tools",
    "$name",
    "a-b",
    "a.b",
    "pkg",
];
