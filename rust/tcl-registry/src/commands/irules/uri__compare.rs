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

//! `URI::compare` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::compare",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Compares two URI's for equality.",
            synopsis: &["URI::compare URI_STRING URI_STRING"],
            snippet: "Compares two URI's as recommended by RFC2616 section 3.2.3.\n\n3.2.3 URI Comparison\n\n   When comparing two URIs to decide if they match or not, a client\n   SHOULD use a case-sensitive octet-by-octet comparison of the entire\n   URIs, with these exceptions:\n\n      - A port that is empty or not given is equivalent to the default\n        port for that URI-reference;\n\n        - Comparisons of host names MUST be case-insensitive;\n\n        - Comparisons of scheme names MUST be case-insensitive;\n\n        - An empty abs_path is equivalent to an abs_path of \"/\".",
            source: "https://clouddocs.f5.com/api/irules/URI__compare.html",
            examples: "when HTTP_REQUEST {\n  set uri_to_check \"/dir1/somepath\"\n  if { [URI::compare [HTTP::uri] $uri_to_check] } {\n    log local0. \"URI's are equal!\"\n  }\n}",
            return_value: "Returns 1 if URIs match; 0 otherwise.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "URI::compare URI_STRING URI_STRING",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
