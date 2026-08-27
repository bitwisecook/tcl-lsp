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

//! `MQTT::session_present` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::session_present",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set session_present flag of MQTT CONNACK message.",
            synopsis: &["MQTT::session_present ('0' | '1')?"],
            snippet: "This command can be used to get or set session_present flag of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNACK",
            source: "https://clouddocs.f5.com/api/irules/MQTT__session_present.html",
            examples: "# Prevent session_present from being 1\nwhen MQTT_SERVER_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n        \"CONNACK\" {\n            if { [MQTT::session_present] == 1 } {\n                MQTT::session_present 0\n            }\n        }\n    }\n}",
            return_value: "When called without an argument, this command returns the session_present flag of MQTT CONNACK message.",
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
            synopsis: "MQTT::session_present ('0' | '1')?",
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
