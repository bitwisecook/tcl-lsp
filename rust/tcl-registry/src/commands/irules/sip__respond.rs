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

//! `SIP::respond` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::respond",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Terminates a SIP response and responds with one of your creation.",
            synopsis: &["SIP::respond RESPONSE_CODE (PHRASE (HEADER_NAME HEADER_VALUE)*)?"],
            snippet: "This command allows you to terminate a SIP request and send a custom\nformatted response directly from the iRule.",
            source: "https://clouddocs.f5.com/api/irules/SIP__respond.html",
            examples: "when SIP_REQUEST {\n  log local0. [SIP::uri]\n  log local0. [SIP::header Via 0]\n  if {[SIP::method] == \"INVITE\"} {\n    SIP::respond 401 \"no way\" X-Header \"xxx here\"\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "SIP::respond RESPONSE_CODE (PHRASE (HEADER_NAME HEADER_VALUE)*)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
