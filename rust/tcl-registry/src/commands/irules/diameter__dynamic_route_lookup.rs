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

//! `DIAMETER::dynamic_route_lookup` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::dynamic_route_lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set whether messages should be routed dynamically.",
            synopsis: &["DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?"],
            snippet: "\"message\":\nIf status is set to \"enabled\", previously created dynamic routes will be consulted during the routing of this message.\n\n\"connection\":\nThe setting will be applied to this and all later messages on this connection.\n\nThe zero-argument form of this command returns whether the setting is enabled on the current message.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__dynamic_route_lookup.html",
            examples: "when DIAMETER_INGRESS {\n                if { ([DIAMETER::header appid] equals 666) } {\n                    DIAMETER::dynamic_route_lookup message disabled\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
