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

//! `MQTT::insert` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::insert",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Insert an MQTT message",
            synopsis: &["MQTT::insert (('before' | 'after') ("],
            snippet: "This command can be used to insert an MQTT message before or after current message.\nSince MQTT_CLIENT_SHUTDOWN event does not have current message only 'MQTT::insert after' is supported for it.\n\nThis command is valid for all MQTT message types:\n\n    CONNECT, CONNACK,\n    PUBLISH, PUBACK, PUBREC, PUBREL, PUBCOMP,\n    SUBSCRIBE, SUBACK,\n    UNSUBSCRIBE, UNSUBACK,\n    PINGREQ, PINGRESP,\n    DISCONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__insert.html",
            examples: "when MQTT_SERVER_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n       \"SUBACK\" {\n          if { [MQTT::packet_id > 1000] } {\n             MQTT::drop\n          }\n       }\n    }\n}",
            return_value: "None.",
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
            synopsis: "MQTT::insert (('before' | 'after') (",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
