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

//! `FLOW::refresh` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::refresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Refresh the flow.",
            synopsis: &["FLOW::refresh ANY_CHARS"],
            snippet: "Updates the last used time on the flow to now.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__refresh.html",
            examples: "when CLIENT_DATA {\n        # Log and refresh the related flow whenever the client sends data.\n        log local0. \"Flow idle duration before refresh [FLOW::idle_duration $result]\"\n        FLOW::refresh $result\n        log local0. \"Flow idle duration after refresh [FLOW::idle_duration $result]\"\n        TCP::release\n        TCP::collect\n\n    }",
            return_value: "",
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
            synopsis: "FLOW::refresh ANY_CHARS",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
