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

//! `HTTP::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::payload",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(HTTP_PAYLOAD),
        hover: Some(HoverSnippet {
            summary: "Queries for or manipulates HTTP payload information.",
            synopsis: &[
                "HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
                "HTTP::payload length",
                "HTTP::payload rechunk",
                "HTTP::payload unchunk",
            ],
            snippet: "Queries for or manipulates HTTP payload (content) information. With\nthis command, you can retrieve content, query for content size, or\nreplace a certain amount of content. The content does not include the\nHTTP headers.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__payload.html",
            examples: "when HTTP_RESPONSE_DATA {\nHTTP::respond 200 content [HTTP::payload]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpBody,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        byte_array_payload: Some(BytePayloadSpec::DEFAULT),
        ..CommandSpec::DEFAULT
    }
}
