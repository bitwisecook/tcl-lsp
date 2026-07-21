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

//! `FLOW::priority` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::priority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set/Get flow's internal packet priority.",
            synopsis: &[
                "FLOW::priority FLOW_PRIORITY",
                "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?",
                "FLOW::priority (ANY_CHARS) (FLOW_PRIORITY)?",
            ],
            snippet: "This command is used to get/set the flow's internal packet priority.\nValid priority is any integer value from 0 to 7.\nSyntax:\nFLOW::priority [TCL handle|clientside|serverside] [priority]\n\nFollowing are the variations of this command:\n\nFLOW::priority\n\n Returns the internal packet priority of current flow.\n\nFlow::priority <priority>\n\n Sets the priority of the current flow's internal packet priority.\n Exception is thrown if priority is outside the allowed range [0-7].\n\nFLOW::priority clientside\n\n Returns the priority of the clientside flow's internal packet priority.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__priority.html",
            examples: "when SERVER_CONNECTED {\n  FLOW::priority serverside 4\n\n  # Alternate way to use the command using the TCL flow handle.\n  # Set priority on both client side and server side flow.\n  FLOW::priority $clientflow 2\n  FLOW::priority [FLOW::this] 2\n}",
            return_value: "Get operation returns an integer between 0-7. Set operation returns nothing. Exception is thrown if the operation cannot be completed.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "FLOW::priority FLOW_PRIORITY",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
