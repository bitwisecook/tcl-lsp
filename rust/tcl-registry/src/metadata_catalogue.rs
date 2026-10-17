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

//! Authored metadata variant spellings, independent of Tcl name identity.

/// Comparison policy selected by the actual metadata field owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataSpellingComparison {
    /// Preserve the field's exact authored variant spelling.
    Exact,
    /// ASCII-only case-insensitive metadata labels, without Unicode folding.
    AsciiInsensitive,
}

/// Existing Rust Debug spelling used by Registry metadata catalogues.
/// This is presentation/authoring metadata, never a Tcl command or cell name.
pub fn metadata_variant_name<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// Select an actual catalogue member under the field's explicit comparison.
/// Unknown labels remain unavailable; there is no prefix or whitespace repair.
pub fn metadata_variant_by_name<T: Copy + std::fmt::Debug>(
    all: &[T],
    name: &str,
    comparison: MetadataSpellingComparison,
) -> Option<T> {
    all.iter().copied().find(|value| {
        let spelling = metadata_variant_name(value);
        match comparison {
            MetadataSpellingComparison::Exact => spelling == name,
            MetadataSpellingComparison::AsciiInsensitive => spelling.eq_ignore_ascii_case(name),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_policies_preserve_exact_and_ascii_only_boundaries() {
        use crate::ArgRole;
        assert_eq!(
            metadata_variant_by_name(ArgRole::ALL, "VarWrite", MetadataSpellingComparison::Exact),
            Some(ArgRole::VarWrite)
        );
        assert_eq!(
            metadata_variant_by_name(ArgRole::ALL, "varwrite", MetadataSpellingComparison::Exact),
            None
        );
        assert_eq!(
            metadata_variant_by_name(
                ArgRole::ALL,
                "varwrite",
                MetadataSpellingComparison::AsciiInsensitive
            ),
            Some(ArgRole::VarWrite)
        );
        for name in [
            " VarWrite",
            "VarWrite ",
            "Var",
            "VarWrıte",
            "VarWrite\0tail",
        ] {
            assert_eq!(
                metadata_variant_by_name(
                    ArgRole::ALL,
                    name,
                    MetadataSpellingComparison::AsciiInsensitive
                ),
                None
            );
        }
        assert_eq!(
            metadata_variant_by_name(
                crate::ScriptTiming::ALL,
                "Deferred",
                MetadataSpellingComparison::Exact
            ),
            Some(crate::ScriptTiming::Deferred)
        );
        assert_eq!(
            metadata_variant_by_name(
                crate::ScriptTiming::ALL,
                "deferred",
                MetadataSpellingComparison::Exact
            ),
            None
        );
    }
}
