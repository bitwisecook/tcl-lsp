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

//! `URI::encode` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::encode",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns an encoded version of a given URI.",
            synopsis: &["URI::encode URI_STRING"],
            snippet: "Returns the encoded version of the given URI.\nFor details on URI encoding, see RFC3986, section 2.1. Percent-Encoding.\n\nThis command is equivalent to the BIG-IP 4.X variable encode_uri.",
            source: "https://clouddocs.f5.com/api/irules/URI__encode.html",
            examples: "when HTTP_REQUEST {\n  set my_parameter_value \"my URL encoded parameter value with metacharacters (&*@#[])\"\n  log local0. \"The encoded version of \\\"$my_parameter_value\\\" is \\\"[URI::encode $my_parameter_value]\\\"\"\n  HTTP::redirect \"/path?parameter=[URI::encode $my_parameter_value]\"\n}",
            return_value: "Returns an encoded version of a given URI.",
        }),
        // URL-encodes its input (and strips CR/LF);
        // re-encoding a URL-encoded value double-encodes (T106).
        taint_transform: Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::URL_ENCODED),
        forms: &[FormSpec {
            synopsis: "URI::encode URI_STRING",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
