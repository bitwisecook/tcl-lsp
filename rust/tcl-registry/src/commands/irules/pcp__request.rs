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

//! `PCP::request` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides access to the data sent in a PCP request.",
            synopsis: &["PCP::request (opcode |"],
            snippet: "This command provides access to the data sent in a PCP (Port Control\nProtocol) request. Access to this data is read-only, and the data in\nthe PCP request cannot be modified via the PCP::request command.",
            source: "https://clouddocs.f5.com/api/irules/PCP__request.html",
            examples: "when PCP_REQUEST {\n     if {[PCP::request opcode] == \"map\" && [PCP::request client-addr] == \"192.168.1.1\" } {\n         log \"Received PCP map request for port [PCP::request internal-port] from 192.168.1.1\"\n     }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["PCP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "PCP::request (opcode |",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
