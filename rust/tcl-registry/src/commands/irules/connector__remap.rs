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

//! `CONNECTOR::remap` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::remap",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set client/server IP/Port from connector.",
            synopsis: &[
                "CONNECTOR::remap server_addr IP_ADDR",
                "CONNECTOR::remap client_addr IP_ADDR",
                "CONNECTOR::remap client_port PORT",
                "CONNECTOR::remap server_port PORT",
            ],
            snippet: "CONNECTOR::remap client_addr\n    Set the client IP address from connector profile.\nCONNECTOR::remap server_addr\n    Set the server IP address from connector profile.\nCONNECTOR::remap client_port\n    Set the client port from connector profile.\nCONNECTOR::remap server_port\n    Set the server port from connector profile.",
            source: "https://clouddocs.f5.com/api/irules/CONNECTOR__remap.html",
            examples: "when CONNECTOR_OPEN {\n                if {([CONNECTOR::profile] eq \"/Common/connector_profile_1\")} {\n                    CONNECTOR::remap client_addr 10.10.10.2\n                    log local0. \"Remap client IP address from connector to 10.10.10.2\"\n                    CONNECTOR::remap client_port 333\n                    log local0. \"Remap client port from connector to 333\"\n                    CONNECTOR::remap server_addr 20.20.20.2",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CONNECTOR::remap server_addr IP_ADDR",
            dialects: None,
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
