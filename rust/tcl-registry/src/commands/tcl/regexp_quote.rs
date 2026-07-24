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

//! `regexp::quote` — regex quoting helper alias (the module name uses a
//! single `_` for `::`, since `::` is not a legal file-name character;
//! contrast `regex__quote.rs`, which doubles the underscore for the
//! shorter `regex::quote` spelling).
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regexp::quote STRING",
    dialects: None,
}];

/// Command spec for `regexp::quote`, one of four recognised spellings of
/// the same regex-metacharacter-quoting idiom (`regexp::quote` /
/// `regex::quote` / `regex_quote` / `re_quote`). It is not a documented
/// core Tcl command: Tcl ships no metacharacter-quoting builtin under any
/// of these names. `re_syntax.n` — the previous, inaccurate `source`
/// citation this spec carried — documents regexp syntax only and never
/// mentions a quoting command or utility, confirmed directly against Tcl
/// 8.4, 8.5, 8.6, 9.0, and 9.1 alike; `regexp(n)`'s own `SEE ALSO` lists
/// only `re_syntax`/`regsub` in 8.4, growing to `re_syntax`/`regsub`/
/// `string` from 8.5 onward (8.5, 8.6, 9.0, 9.1 alike), and a dedicated
/// `quote.html`/`.htm` 404s on every tree (8.4-9.1). `regex::quote` is
/// the canonical spelling and the one the T103 (regex-injection) quick
/// fix actually generates and inserts
/// (`tcl_lsp_core::code_actions::REGEX_QUOTE_PROC`: `proc regex::quote
/// {str} { regsub -all {[][{}()*+?.\\^$|]} $str {\\&} }`, arity exactly
/// one); this spec lets the taint analyser recognise a user proc under
/// this alternate name — or one of its three siblings — as the same
/// REGEX_LITERAL-quoting idiom (`tcl_compiler`'s
/// `regexp_quote_suppresses_t103` test) wherever it is called, standard
/// Tcl or otherwise. `dialects: ALL_TCL` (no `IRULES` bit) is deliberate,
/// not an oversight: iRules excludes all four spellings, and this group's
/// omission of the `IRULES` bit is exactly what enforces that — an
/// `ALL_TCL` group never intersects the bare `IRULES` availability mask,
/// so there is no separate disable list. Folding `IRULES` into a dialect
/// union here would re-admit exactly the commands iRules means to exclude.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp::quote",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Escape regex metacharacters in a string.",
            synopsis: &["regexp::quote STRING"],
            snippet: "Returns *STRING* with all regular-expression\nmetacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\nbackslash-escaped so it can be used as a literal\npattern in ``regexp`` or ``regsub``.  Alias for\n``regex::quote``.",
            source: "",
            examples: "set safe_pattern [regexp::quote $user_input]\nif {[regexp $safe_pattern $haystack]} { ... }",
            return_value: "Returns a regex-escaped string.",
        }),
        // Regex-escaped literal output; double-encode → T106.
        taint_transform: Some(TaintColour::REGEX_LITERAL),
        taint_double_encode_colour: Some(TaintColour::REGEX_LITERAL),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
