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

//! `FLOW::this` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::this",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the TCL handle for the current flow.",
            synopsis: &["FLOW::this"],
            snippet: "Returns the TCL handle for the current flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__this.html",
            examples: "when CLIENT_ACCEPTED {\n    set cf [FLOW::this]\n    log local0. \"Current flow is $cf\"\n    unset cf\n}",
            return_value: "TCL handle for the current flow. On error an exception is thrown with a message indicating the cause of failure.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_DATA",
                "LB_SELECTED",
                "SA_PICKED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "FLOW::this",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
