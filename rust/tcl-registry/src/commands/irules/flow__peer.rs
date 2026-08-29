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

//! `FLOW::peer` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::peer",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the TCL flow handle for the peer flow.",
            synopsis: &["FLOW::peer ANY_CHARS"],
            snippet: "Returns the TCL flow handle for the peer flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__peer.html",
            examples: "when SERVER_CONNECTED {\n    # Get server side flow handle.\n    set cf [FLOW::this]\n\n    # Get client side flow handle.\n    set peer [FLOW::peer $cf]\n    log local0. \"Peer flow is $peer\"\n    unset cf peer\n}",
            return_value: "TCL handle for the peer flow. On error an exception is thrown with a message indicating the cause of failure.",
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
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "FLOW::peer ANY_CHARS",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
