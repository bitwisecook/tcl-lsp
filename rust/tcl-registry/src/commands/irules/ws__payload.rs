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

//! `WS::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries for or manipulates Websocket frame payload information.",
            synopsis: &[
                "WS::payload (LENGTH | (OFFSET LENGTH))?",
                "WS::payload length",
                "WS::payload replace OFFSET LENGTH STRING",
            ],
            snippet: "WS::payload <length>\n    Returns the content that the WS::collect command has collected thus far, up to the number of bytes specified. If you do not specify a size, the system returns the entire collected content.\n\nWS::payload <offset> <length>\n    Returns the content that the WS::collect command has collected thus far from the specified offset, up to the number of bytes specified.\n\nWS::payload length\n    Returns the size of the content that has been collected thus far, in bytes.",
            source: "https://clouddocs.f5.com/api/irules/WS__payload.html",
            examples: "when WS_CLIENT_FRAME {\n    WS::collect frame 1000\n    set clen 1000\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "WS::payload (LENGTH | (OFFSET LENGTH))?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
