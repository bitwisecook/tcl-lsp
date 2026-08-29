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

//! `FLOW::idle_timeout` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::idle_timeout",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets/Gets the idle timeout on the flow",
            synopsis: &["FLOW::idle_timeout (ANY_CHARS) (NONNEGATIVE_INTEGER)?"],
            snippet: "Sets/Gets the idle timeout on the flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__idle_timeout.html",
            examples: "when SERVER_CONNECTED {\n    set cf [FLOW::this]\n\n    #Get flow idletimeout\n    log local0. \"Idle timeout: [FLOW::idle_timeout $cf]\"\n\n    #Set flow idletimeout\n    FLOW::idle_timeout $cf 100\n\n    unset cf\n}",
            return_value: "Set operation: Nothing is returned Get operation: Idle timeout set on the flow as number string. On error an exception is thrown with a message indicating the cause of failure.",
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
            synopsis: "FLOW::idle_timeout (ANY_CHARS) (NONNEGATIVE_INTEGER)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
