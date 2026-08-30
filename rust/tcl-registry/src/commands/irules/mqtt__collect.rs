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

//! `MQTT::collect` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::collect",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Collect the specified amount of MQTT message payload data",
            synopsis: &["MQTT::collect (COLLECT)?"],
            snippet: "Collects the specified amount of MQTT message payload data before triggering a MQTT_CLIENT_DATA or MQTT_SERVER_DATA event.\n\nWhen collecting data in a clientside event, the MQTT_CLIENT_DATA event will be triggered.\nWhen collecting data in a serverside event, the MQTT_SERVER_DATA event will be triggered.\n\nThis command is valid only for following MQTT message types:\n\n    PUBLISH\n\nThis command allows you to perform various operations on MQTT PUBLISH message like modify its contents.\nNOTE: Please make sure that MQTT PUBLISH message expects to receive a payload by using [MQTT::payload length].",
            source: "https://clouddocs.f5.com/api/irules/MQTT__collect.html",
            examples: "when MQTT_CLIENT_DATA {\n   set type [MQTT::type]\n   switch $type {\n       \"PUBLISH\" {\n          set payload [MQTT::payload]\n          MQTT::release\n          set found [class match $payload contains blacklisted_keywords_datagroup]\n          if { $found != \"\" } {\n              MQTT::disconnect\n          }\n       }\n   }\n}",
            return_value: "",
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
            synopsis: "MQTT::collect (COLLECT)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        data_collection: Some(MQTT_COLLECT),
        ..CommandSpec::DEFAULT
    }
}
