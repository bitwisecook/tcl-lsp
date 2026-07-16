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

//! `regex::quote` — regex quoting helper alias (`::` spelling).
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regex::quote STRING",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regex::quote",
        dialects: None,
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Escape regex metacharacters in a string.",
            synopsis: &["regex::quote STRING", "regex::quote string"],
            snippet: "Returns *STRING* with all regular-expression\nmetacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\nbackslash-escaped so it can be used as a literal\npattern in ``regexp`` or ``regsub``.",
            source: "Tcl man page re_syntax.n",
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
