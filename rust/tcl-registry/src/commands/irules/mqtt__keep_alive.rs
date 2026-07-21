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

//! `MQTT::keep_alive` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::keep_alive",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set keep_alive field of MQTT CONNECT message.",
            synopsis: &["MQTT::keep_alive (KEEP_ALIVE)?"],
            snippet: "This command can be used to get or set keep_alive field of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__keep_alive.html",
            examples: "# Increase keep-alive to at least 60 seconds\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n       \"CONNECT\"  {\n           if { [MQTT::keep_alive] < 60} {\n              MQTT::keep_alive 60\n           }\n       }\n   }\n}",
            return_value: "When called without an argument, this command returns the keep_alive field of MQTT CONNECT message.",
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
            kind: FormKind::Default,
            synopsis: "MQTT::keep_alive (KEEP_ALIVE)?",
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
