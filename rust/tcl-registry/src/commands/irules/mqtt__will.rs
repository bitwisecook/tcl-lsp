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

//! `MQTT::will` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::will",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set will-topic, will-message, will-qos, and will-retain fields of MQTT CONNECT message.",
            synopsis: &["MQTT::will (('topic' (TOPIC)?) |"],
            snippet: "This command can be used to get or set will-topic, will-message, will-qos, and will-retain fields of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    CONNECT",
            source: "https://clouddocs.f5.com/api/irules/MQTT__will.html",
            examples: "# Enforce a mandatary default will message, if will is not present in connect\nwhen MQTT_CLIENT_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n        \"CONNECT\" {\n            if { [MQTT::will topic] == \"\" } {\n                MQTT::will topic \"/bigip/default/will/[MQTT::username]/[MQTT::client_id]/[client_addr]\"\n                MQTT::will message \"client disconnected without sending DISCONNECT message\"\n                MQTT::will qos 0\n                MQTT::will retain 0\n            }",
            return_value: "When called without an argument, each of the sub-commands return the will-topic, will-message, will-qos, or will-retain field of MQTT CONNECT message.",
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
            synopsis: "MQTT::will (('topic' (TOPIC)?) |",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
