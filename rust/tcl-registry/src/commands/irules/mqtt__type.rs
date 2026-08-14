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

//! `MQTT::type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get type of MQTT message",
            synopsis: &["MQTT::type"],
            snippet: "This command can be used to get type of MQTT message.\nThis command is valid for all MQTT message types:\n\n    CONNECT, CONNACK,\n    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n    SUBSCRIBE, SUBACK,\n    UNSUBSCRIBE, UNSUBACK,\n    PINGREQ, PINGRESP,\n    DISCONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__type.html",
            examples: "# Typical usage pattern...\n#\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n      \"CONNECT\" {\n         # Do connect processing\n      }\n      \"SUBSCRIBE\" {\n         # Do subscribe processing\n      }\n      \"PUBLISH\" {\n         # Do publish processing\n      }\n   }\n}",
            return_value: "A string representation of MQTT message types:",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MQTT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "MQTT::type",
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
