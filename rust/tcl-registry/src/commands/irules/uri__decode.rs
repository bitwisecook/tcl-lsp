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

//! `URI::decode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::decode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a decoded version of a given URI.",
            synopsis: &["URI::decode URI_STRING"],
            snippet: "Returns a URI decoded version of a given URI.\nFor details on URI encoding, see RFC3986, section 2.1. Percent-Encoding.\n\nThis command is equivalent to the BIG-IP 4.X variable decode_uri.",
            source: "https://clouddocs.f5.com/api/irules/URI__decode.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"The decoded version of \\\"[HTTP::query]\\\" is \\\"[URI::decode [HTTP::query]]\\\"\"\n}",
            return_value: "Returns a decoded version of a given URI.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "URI::decode URI_STRING",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        traits: Traits::IS_UNESCAPE,
        ..CommandSpec::DEFAULT
    }
}
