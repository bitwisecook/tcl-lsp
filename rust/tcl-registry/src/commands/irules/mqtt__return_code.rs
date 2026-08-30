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

//! `MQTT::return_code` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::return_code",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set return-code field of MQTT CONNACK message.",
            synopsis: &["MQTT::return_code (RETURN_CODE)?"],
            snippet: "This command can be used to get or set return-code field of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNACK",
            source: "https://clouddocs.f5.com/api/irules/MQTT__return_code.html",
            examples: "# For security reasons convert all refused reasons to 5\nwhen MQTT_SERVER_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n       \"CONNACK\" {\n          if { [MQTT::return_code] != 0 } {\n             MQTT::return_code 5\n          }\n       }\n   }\n}",
            return_value: "When called without an argument, this command returns the return-code field of MQTT CONNACK message.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MQTT"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "MQTT::return_code (RETURN_CODE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
