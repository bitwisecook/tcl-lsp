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

//! `re_quote` — regex quoting helper alias (underscore spelling).
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "re_quote STRING",
    ..FormSpec::DEFAULT
}];

/// Command spec for `re_quote`, one of four recognised spellings of the
/// same regex-metacharacter-quoting idiom (`re_quote` / `regex_quote` /
/// `regex::quote` / `regexp::quote`). None of the four is a documented
/// core Tcl command: Tcl ships no metacharacter-quoting builtin, and
/// `re_quote.html`/`.htm` 404s on every tcl-lang.org manpage tree for
/// 8.4, 8.5, 8.6, 9.0, and 9.1 alike; `re_syntax.n` (the previous,
/// inaccurate `source` citation below) documents regexp syntax only and
/// never mentions a `re_quote` command or any quoting utility.
/// `regex::quote` is the spelling the T103 (regex-injection) quick fix
/// actually generates and inserts
/// (`tcl_lsp_core::code_actions::REGEX_QUOTE_PROC`); this spec — like its
/// three siblings — lets the taint analyser recognise a user proc under
/// this alternate name as the same REGEX_LITERAL-quoting idiom
/// (`tcl_compiler`'s `re_quote_suppresses_t103` test) wherever it is
/// called, standard Tcl or otherwise.
/// `surface: Some(SpecSurface::ALL_TCL)` is deliberate, not an oversight:
/// iRules excludes all four spellings, and that exclusion now comes
/// straight from this field — the `ALL_TCL` group carries no `IRULES`
/// bit, so it never intersects the bare `IRULES` mask, and there is no
/// disable list.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "re_quote",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Escape regex metacharacters in a string.",
            synopsis: &["re_quote STRING"],
            snippet: "Returns *STRING* with all regular-expression\nmetacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\nbackslash-escaped so it can be used as a literal\npattern in ``regexp`` or ``regsub``.  Alias for\n``regex::quote``.",
            source: "",
            examples: "set safe_pattern [re_quote $user_input]\nif {[regexp $safe_pattern $haystack]} { ... }",
            return_value: "Returns a regex-escaped string.",
        }),
        // Output is a regex-escaped literal; re-quoting an
        // already-escaped value double-encodes it (T106).
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
