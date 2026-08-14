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

//! `HTML::encode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "HTML-encodes a string, escaping special characters.",
            synopsis: &["HTML::encode STRING"],
            snippet: "Replaces HTML-special characters (``<``, ``>``, ``&``,\n``\"``, ``'``) with their entity equivalents so the\nstring is safe to embed in an HTML text context.",
            source: "https://clouddocs.f5.com/api/irules/",
            examples: "when HTTP_REQUEST {\n  set user_input [HTTP::query]\n  HTTP::respond 200 content \"<p>[HTML::encode $user_input]</p>\"\n}",
            return_value: "Returns an HTML-escaped string.",
        }),
        // HTML-escapes its input (and strips CR/LF);
        // re-encoding an HTML-escaped value double-encodes (T106).
        taint_transform: Some(TaintColour::HTML_ESCAPED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::HTML_ESCAPED),
        forms: &[FormSpec {
            synopsis: "HTML::encode STRING",
            ..FormSpec::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
