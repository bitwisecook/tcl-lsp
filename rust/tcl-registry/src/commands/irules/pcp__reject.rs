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

//! `PCP::reject` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PCP::reject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides the ability to cause a PCP reqeust to fail based on processing in the iRule.",
            synopsis: &["PCP::reject PCP_RESULT_CODE"],
            snippet: "This command provides the ability to cause a PCP (Port Control\nProtocol) reqeust to fail based on processing in the iRule. If the\nreject command is issued, the PCP request is rejected with the\nspecified result code and no other action is taken by the PCP proxy.",
            source: "https://clouddocs.f5.com/api/irules/PCP__reject.html",
            examples: "when PCP_REQUEST {\n     if {[PCP::request opcode] == \"map\" &&\n             [PCP::request internal-port] == 22 } {\n         log \"Rejecting PCP request to map SSH\"\n         PCP::reject 1\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["PCP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PCP::reject PCP_RESULT_CODE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
