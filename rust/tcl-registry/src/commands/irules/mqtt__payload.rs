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

//! `MQTT::payload` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "length",
        arity: Arity::exact(0),
        detail: "Get payload length.",
        synopsis: "MQTT::payload length",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::new(1, 3),
        detail: "Replace payload data.",
        synopsis: "MQTT::payload replace <data> ?offset? ?length?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prepend",
        arity: Arity::exact(1),
        detail: "Prepend data to payload.",
        synopsis: "MQTT::payload prepend <data>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "append",
        arity: Arity::exact(1),
        detail: "Append data to payload.",
        synopsis: "MQTT::payload append <data>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

const MQTT_REQUIREMENTS: EventRequires = EventRequires {
    client_side: false,
    server_side: false,
    transport: None,
    profiles: &["MQTT"],
    also_in: &[],
    init_only: false,
    flow: false,
    capability: None,
};

const MQTT_MESSAGE_EVENTS: &[&str] = &[
    "MQTT_CLIENT_INGRESS",
    "MQTT_SERVER_INGRESS",
    "MQTT_CLIENT_DATA",
    "MQTT_SERVER_DATA",
];
const MQTT_DATA_EVENTS: &[&str] = &["MQTT_CLIENT_DATA", "MQTT_SERVER_DATA"];
const EVENT_REQUIREMENT_FORMS: &[EventRequirementForm] = &[
    EventRequirementForm {
        argument_prefix: &["length"],
        requires: Some(MQTT_REQUIREMENTS),
        only_in: MQTT_MESSAGE_EVENTS,
    },
    EventRequirementForm {
        argument_prefix: &["replace"],
        requires: Some(MQTT_REQUIREMENTS),
        only_in: MQTT_MESSAGE_EVENTS,
    },
    EventRequirementForm {
        argument_prefix: &["prepend"],
        requires: Some(MQTT_REQUIREMENTS),
        only_in: MQTT_MESSAGE_EVENTS,
    },
    EventRequirementForm {
        argument_prefix: &["append"],
        requires: Some(MQTT_REQUIREMENTS),
        only_in: MQTT_DATA_EVENTS,
    },
    EventRequirementForm {
        argument_prefix: &[],
        requires: Some(MQTT_REQUIREMENTS),
        only_in: MQTT_DATA_EVENTS,
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Manipulate payload of MQTT PUBLISH message",
            synopsis: &[
                "MQTT::payload ?subcommand? ?args?",
                "MQTT::payload length",
                "MQTT::payload replace <data> ?offset? ?length?",
            ],
            snippet: "This command can be used to manipulate payload of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH",
            source: "https://clouddocs.f5.com/api/irules/MQTT__payload.html",
            examples: "#Example: Redirect PUBLISH that has payloads with blocked keywords defined in\n#blacklisted_keywords_datagroup in first 200 bytes. Prepend a admin message in\n#the payload.\n#\nwhen MQTT_CLIENT_INGRESS {\n    set type [MQTT::type]\n    switch $type {\n       \"PUBLISH\" {\n          if { [class exists  blacklisted_keywords_datagroup] } {\n             MQTT::collect 200\n          }\n       }\n    }\n}",
            return_value: "When called without an argument, this command returns the collected payload of MQTT PUBLISH message.",
        }),
        event_requires: Some(MQTT_REQUIREMENTS),
        event_requirement_forms: EVENT_REQUIREMENT_FORMS,
        forms: &[FormSpec {
            synopsis: "MQTT::payload ?subcommand? ?args?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        byte_array_payload: Some(BytePayloadSpec {
            replace_data_index: 1,
            ..BytePayloadSpec::DEFAULT
        }),
        data_collection: Some(MQTT_PAYLOAD),
        ..CommandSpec::DEFAULT
    }
}
