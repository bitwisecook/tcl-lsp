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

//! `PCP::response` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::response",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides access to the data in a PCP response packet.",
            synopsis: &["PCP::response (opcode |"],
            snippet: "This command provides access to the data in a PCP (Port Control\nProtocol) response packet. Access to this data is read-only, and the\ndata in the PCP response cannot be modified via the PCP::response\ncommand.",
            source: "https://clouddocs.f5.com/api/irules/PCP__response.html",
            examples: "when PCP_RESPONSE {\n    if {[PCP::response opcode] == \"map\" && [PCP::response result] != 0] } {\n        log \"PCP map request from\\\n              [PCP::response client-addr]:[PCP::response internal-port]\\\n              failed with a result of [PCP::response result]\"\n    }\n}",
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
            synopsis: "PCP::response (opcode |",
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
