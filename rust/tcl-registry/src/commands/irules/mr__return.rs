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

//! `MR::return` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::return",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the current message to the originating connection.",
            synopsis: &[
                "MR::return",
                "MR::return (no_route_found | queue_full | no_connection | connection_closing | internal_error | max_retries_exceeded )",
            ],
            snippet: "The MR::return command instructs the Message Routing Framework to return the current message to the originating connection. The message's route status will be updated to 'returned by irule' or the provided route status. When the connection is received on the originating connection, MR_FAILED event will be raised.\n        \nReturns the current message to the originating connection with a route status of 'returned by irule'\n            \nReturns the current message to the originating connection and sets the route status to the route status specified.",
            source: "https://clouddocs.f5.com/api/irules/MR__return.html",
            examples: "when MR_INGRESS {\n    if {[DIAMETER::is_response]} {\n        incr pend_req -1\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "MR::return",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
