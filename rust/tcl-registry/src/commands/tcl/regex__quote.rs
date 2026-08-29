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

//! `regex::quote` — canonical regex-quoting helper (`re_quote`,
//! `regex_quote`, and `regexp::quote` are recognised aliases of the same
//! idiom, `::` being the canonical spelling).
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "regex::quote STRING",
    ..FormSpec::DEFAULT
}];

/// Command spec for `regex::quote`, the canonical spelling of the
/// regex-metacharacter-quoting idiom this registry recognises under four
/// names (`regex::quote` / `re_quote` / `regex_quote` / `regexp::quote`).
/// It is not a documented core Tcl command: Tcl ships no
/// metacharacter-quoting builtin. `re_syntax.n` (the previous, inaccurate
/// `source` citation below) documents regexp syntax only and never
/// mentions a quoting command or utility, re-read directly against Tcl
/// 8.4, 8.5, 8.6, 9.0, and 9.1 alike. The `TclCmd` alphabetical command
/// index likewise lists no `quote`- or `regex`-prefixed command (only
/// `regexp`/`regsub`/`re_syntax`) on all five trees: `contents.htm` for
/// 8.4/8.5/8.6 directly, and the bare `TclCmd/` directory listing for
/// 9.0 and 9.1, since `contents.htm`/`.html` both 404 on those two
/// trees specifically (an index-page-only gotcha, distinct from the
/// page-level 8.6-`.htm`-not-`.html` redirect noted in
/// `re_quote.rs`/`regex_quote.rs`). A dedicated `quote.html`/`.htm`
/// manpage likewise 404s on all five trees alike — 8.4, 8.5, 8.6, 9.0,
/// and 9.1 — consistent with the `re_quote.html`/`.htm` 404 already
/// recorded for the underscore spelling in `re_quote.rs`. `regex::quote`
/// is the spelling the T103 (regex-injection) quick fix actually
/// generates and inserts (`tcl_lsp_core::code_actions::REGEX_QUOTE_PROC`:
/// `proc regex::quote {str} { regsub -all {[][{}()*+?.\\^$|]} $str
/// {\\&} }`, arity exactly one); this spec lets the taint analyser
/// recognise a user proc under this name — or one of its three
/// siblings — as the same REGEX_LITERAL-quoting idiom (`tcl_compiler`'s
/// `regex_colon_quote_suppresses_t103` test, plus the `regex_literal`
/// and `t106_double_encoding` test modules) wherever it is called,
/// standard Tcl or otherwise.
/// `surface: Some(SpecSurface::ALL_TCL)` is deliberate, not an oversight:
/// iRules excludes all four spellings, and that exclusion now comes
/// straight from this field — the `ALL_TCL` group carries no `IRULES`
/// bit, so it never intersects the bare `IRULES` mask, and there is no
/// disable list.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regex::quote",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Escape regex metacharacters in a string.",
            synopsis: &["regex::quote STRING"],
            snippet: "Returns *STRING* with all regular-expression\nmetacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\nbackslash-escaped so it can be used as a literal\npattern in ``regexp`` or ``regsub``.",
            source: "",
            examples: "set safe_pattern [regex::quote $user_input]\nif {[regexp $safe_pattern $haystack]} { ... }",
            return_value: "Returns a regex-escaped string.",
        }),
        // Regex-escaped literal output; double-encode → T106.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
