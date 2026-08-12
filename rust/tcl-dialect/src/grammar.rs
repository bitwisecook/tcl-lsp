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

//! Dialect-derived lexing grammar knobs.
//!
//! [`BracedVarStyle`] and [`LexerGrammar`] live here — below `tcl-lexer` —
//! so the lexer's `LexerConfig` and the `DialectProfile` catalog share one
//! definition of the per-dialect grammar surface instead of keeping parallel
//! string-keyed tables.

/// The dialect's `${…}` variable-name delimiting rule — Tcl 9.0 changed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BracedVarStyle {
    /// Tcl 9.x (and the unversioned default): `Tcl_ParseVarName` tracks
    /// nested `{…}` pairs and treats `\X` as an inert two-character unit
    /// inside the braces, so `${a{b}c}` names the variable `a{b}c`
    /// (tcl9.0.1 `tclParse.c`, the `braceCount` loop).
    #[default]
    Tcl9Nesting,
    /// The 8.x family (8.4–8.6, iRules/iApps, EDA): the name runs to the
    /// FIRST literal `}` — no nesting, no backslash processing — so
    /// `${a{b}c}` names `a{b` and `c}` is ordinary word text
    /// (8.6.14 `tclParse.c:1466`, tclsh-verified).
    FirstClose,
}

impl BracedVarStyle {
    /// Whether this style tracks nested `{…}` / `\X` pairs (the Tcl 9 rule).
    #[must_use]
    pub fn nests(self) -> bool {
        matches!(self, Self::Tcl9Nesting)
    }
}

/// The dialect-derived slice of the lexer configuration — exactly the fields
/// of `tcl_lexer::LexerConfig` that vary *by dialect* (as opposed to the
/// call-site knobs: strict quoting, sub-lexing base offsets).
///
/// A `DialectProfile` carries one of these; `LexerConfig` is built from it.
/// The profile's copy is the single source of these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexerGrammar {
    /// When true, `{*}` at a word boundary followed by a non-separator is
    /// argument expansion (TIP 157). True for Tcl 8.5+ and dialects built on
    /// 8.5+; false for Tcl 8.4 and iRules.
    pub expand_syntax: bool,
    /// When true, `}{` at a brace-string boundary separates two words
    /// (a zero-width ghost separator). iRules-only.
    pub irules_brace_separator: bool,
    /// How a `${…}` variable name is delimited — see [`BracedVarStyle`].
    pub braced_var: BracedVarStyle,
    /// When true, the script reader skips a leading UTF-8 byte-order mark
    /// (U+FEFF) at offset 0 of a *file* before evaluating it.
    ///
    /// Tcl 9.0's `source` does (`Tcl_FSEvalFileEx` strips the BOM after
    /// decoding); Tcl 8.x's does not, so an 8.x script whose first byte is a
    /// BOM really does fail with `invalid command name "﻿set"` and the
    /// unresolved-command diagnostic on it is correct.  Only the *file* entry
    /// point skips it — a BOM inside a string, or partway through a script,
    /// is ordinary data in every version, which is why this is a grammar
    /// property consulted once at the top of a file analysis rather than a
    /// lexer rule that fires wherever U+FEFF appears.
    pub script_skips_leading_bom: bool,
}

impl Default for LexerGrammar {
    /// The modern-Tcl (9.x / unversioned) grammar — matches
    /// `LexerConfig::default()`.
    fn default() -> Self {
        Self {
            expand_syntax: true,
            irules_brace_separator: false,
            braced_var: BracedVarStyle::Tcl9Nesting,
            script_skips_leading_bom: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BracedVarStyle, LexerGrammar};

    #[test]
    fn nesting_rule_is_tcl9_only() {
        assert!(BracedVarStyle::Tcl9Nesting.nests());
        assert!(!BracedVarStyle::FirstClose.nests());
        assert_eq!(BracedVarStyle::default(), BracedVarStyle::Tcl9Nesting);
    }

    #[test]
    fn default_grammar_is_modern_tcl() {
        let g = LexerGrammar::default();
        assert!(g.expand_syntax);
        assert!(!g.irules_brace_separator);
        assert_eq!(g.braced_var, BracedVarStyle::Tcl9Nesting);
        assert!(g.script_skips_leading_bom);
    }
}
