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

//! `regsub` helper aliases and regex quoting commands.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "re_quote STRING",
}];

/// Command spec for Tcl `re_quote` (regex quoting helper).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "re_quote",
        dialects: None,
        traits: Traits::PURE,
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Escape regex metacharacters in a string.",
            synopsis: &["re_quote STRING", "re_quote string"],
            snippet: "Returns *STRING* with all regular-expression\nmetacharacters backslash-escaped so it can be\nused as a literal pattern in ``regexp`` or\n``regsub``.  Alias for ``regex::quote``.",
            source: "Tcl man page re_syntax.n",
            examples: "",
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
