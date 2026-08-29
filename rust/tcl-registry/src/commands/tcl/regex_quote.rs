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

//! `regex_quote` — regex quoting helper alias (underscore spelling).
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "regex_quote STRING",
    ..FormSpec::DEFAULT
}];

/// Command spec for `regex_quote`, one of four recognised spellings of the
/// same regex-metacharacter-quoting idiom (`regex_quote` / `re_quote` /
/// `regex::quote` / `regexp::quote`). None of the four is a documented
/// core Tcl command: Tcl ships no metacharacter-quoting builtin, and
/// `regex_quote.html` (`.htm` on the 8.6 tree, whose `.html` redirects to
/// a broken `Location` header) 404s on every tcl-lang.org manpage tree for
/// 8.4, 8.5, 8.6, 9.0, and 9.1 alike; the `TclCmd` alphabetical command
/// index likewise lists no `quote`- or `regex`-prefixed command (only
/// `regexp`/`regsub`) on any of the five trees — `contents.htm` for
/// 8.4/8.5/8.6, and `index.html` (reached from `regexp.html`'s own nav
/// link, since `contents.htm`/`.html` both 404 there) for 9.0/9.1.
/// `re_syntax.n` (the previous,
/// inaccurate `source` citation below) documents regexp syntax only and
/// never mentions a `regex_quote` command or any quoting utility.
/// `regex::quote` is the spelling the T103 (regex-injection) quick fix
/// actually generates and inserts
/// (`tcl_lsp_core::code_actions::REGEX_QUOTE_PROC`); this spec — like its
/// three siblings — lets the taint analyser recognise a user proc under
/// this alternate name as the same REGEX_LITERAL-quoting idiom wherever it
/// is called, standard Tcl or otherwise. `tcl_compiler`'s `taint_depth.rs`
/// covers the other three spellings by name
/// (`regexp_quote_suppresses_t103`, `regex_colon_quote_suppresses_t103`,
/// `re_quote_suppresses_t103`) but has no test yet exercising this literal
/// underscore-without-colon spelling. `surface: ALL_TCL` (no `IRULES`
/// bit) is deliberate, not an oversight: iRules excludes all four
/// spellings, and this group's omission of the `IRULES` bit is exactly
/// what enforces that — an `ALL_TCL` group never intersects the bare
/// `IRULES` availability mask, so there is no separate disable list.
/// Folding `IRULES` into a dialect union here would re-admit exactly the
/// commands iRules means to exclude.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regex_quote",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Escape regex metacharacters in a string.",
            synopsis: &["regex_quote STRING"],
            snippet: "Returns *STRING* with all regular-expression\nmetacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\nbackslash-escaped so it can be used as a literal\npattern in ``regexp`` or ``regsub``.  Alias for\n``regex::quote``.",
            source: "",
            examples: "set safe_pattern [regex_quote $user_input]\nif {[regexp $safe_pattern $haystack]} { ... }",
            return_value: "Returns a regex-escaped string.",
        }),
        // Regex-escaped literal output; double-encode → T106.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
