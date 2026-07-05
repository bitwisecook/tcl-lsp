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

//! The combined `f5 query --help-manual` surface.
//!
//! Concatenates the grammar reference, the builtins catalogue, and the
//! cookbook, composing the three sections from the [`crate::grammar`],
//! [`crate::builtins`], and [`crate::examples`] formatters. The builtins
//! section is the metadata-driven catalogue (see
//! [`crate::builtins::format_catalogue`]) rather than per-function
//! prose.

/// Render the comprehensive manual: grammar + builtins + cookbook.
#[must_use]
pub fn format_manual() -> String {
    let mut out = String::new();
    out.push_str(&crate::grammar::format_grammar());
    out.push('\n');
    out.push_str(&crate::builtins::format_catalogue(None));
    out.push('\n');
    out.push_str(&crate::examples::format_examples());
    out
}

#[cfg(test)]
mod tests {
    use super::format_manual;

    #[test]
    fn manual_composes_all_three_sections() {
        let m = format_manual();
        assert!(m.contains("F5 QUERY DSL — GRAMMAR"), "grammar section");
        assert!(
            m.contains("F5 QUERY DSL — BUILTIN FUNCTIONS"),
            "builtins section"
        );
        assert!(m.contains("F5 QUERY DSL — COOKBOOK"), "cookbook section");
        assert!(m.ends_with('\n'));
    }
}
