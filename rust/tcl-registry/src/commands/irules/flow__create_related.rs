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

//! `FLOW::create_related` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::create_related",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a related client side and server side flow.",
            synopsis: &[
                "FLOW::create_related (((-translation-loose) (-hairpin))#)? (FLOW_CREATE_RELATED_SUBCMDS)+",
            ],
            snippet: "Creates a related connection. Each related connection has two flows in it, a clientside flow and a serverside flow. The clientside flow is created using\nthe information provided in \"clientflow\" and serverside flow is created using the information provided in the \"serverflow\". Both these flows are linked\ntogether and form a connection. BIGIP excepts that the the first packet always comes from the client side of the connection for all protocols except UDP.\nThe returned TCL handle points to the clientside flow. [FLOW::peer] command can be used to get a handle to the peer flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__create_related.html",
            examples: "when SERVER_CONNECTED {\n            # LSN pool with prefix 4.4.4.0/30,port-range=2000-2005 and NAPT mode is configured. Parent connection is translated as follows\n            # 10.10.0.1%1:60412 -> 10.20.0.1%1:9000 TO 4.4.4.1:1084  10.20.0.1:9000  tcp\n            # Subscriber side: 10.10.0.1%1:60412 -> 10.20.0.1%1:9000\n            # Internet side: 4.4.4.1:1084  10.20.0.1:9000\n            # Below is an example of couple of related connections \n            \n            # Connection-1:",
            return_value: "TCL handle for the client side flow. On error an exception is thrown with a message indicating the cause of failure. The string representation of the TCL handle can be used to retrieve the flow details.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_DATA", "SERVER_CONNECTED", "SERVER_DATA"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "FLOW::create_related (((-translation-loose) (-hairpin))#)? (FLOW_CREATE_RELATED_SUBCMDS)+",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-translation-loose",
                    value: OptionValue::flag(),
                    detail: "Option -translation-loose.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-hairpin",
                    value: OptionValue::flag(),
                    detail: "Option -hairpin.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
