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

//! `MQTT::topic` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "replace",
        arity: Arity::new(1, 2),
        detail: "Replace topic name.",
        synopsis: "MQTT::topic replace <topic> ?index?",
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
        name: "count",
        arity: Arity::exact(0),
        detail: "Get topic count.",
        synopsis: "MQTT::topic count",
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
        name: "list",
        arity: Arity::exact(0),
        detail: "List all topics.",
        synopsis: "MQTT::topic list",
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
        name: "index",
        arity: Arity::exact(1),
        detail: "Get topic at index.",
        synopsis: "MQTT::topic index <n>",
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
        name: "qos",
        arity: Arity::new(0, 1),
        detail: "Get/set topic QoS.",
        synopsis: "MQTT::topic qos ?index?",
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
        name: "add",
        arity: Arity::new(1, 2),
        detail: "Add a topic.",
        synopsis: "MQTT::topic add <topic> ?qos?",
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
        name: "delete",
        arity: Arity::exact(1),
        detail: "Delete a topic.",
        synopsis: "MQTT::topic delete <index>",
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

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::topic",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Manipulate topic(s) of MQTT message.",
            synopsis: &[
                "MQTT::topic ?subcommand? ?args?",
                "MQTT::topic replace <new_topic> ?index?",
                "MQTT::topic count",
            ],
            snippet: "This command can be used to manipulate topic(s) of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH\n    SUBSCRIBE\n    UNSUBSCRIBE",
            source: "https://clouddocs.f5.com/api/irules/MQTT__topic.html",
            examples: "when MQTT_SERVER_INGRESS {\n    set smtype [MQTT::type]\n    if {$smtype == \"SUBACK\"} {\n       set mid [MQTT::packet_id]\n       set tc [table lookup -subtable \"packetid_count_table\" \"[client_addr]_[client_port]_${mid}\"]\n       set return_codes [MQTT::return_code_list]\n       set return_codes [lreplace $return_codes $tc $tc]\n       MQTT::replace type SUBACK packet_id $mid return_code_list $return_codes\n    }\n}",
            return_value: "When called without an argument, this command returns the topic-name of MQTT PUBLISH message, or the topic-name of the first topic of MQTT SUBSCRIBE and UNSUBSCRIBE messages.",
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
            synopsis: "MQTT::topic ?subcommand? ?args?",
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
        ..CommandSpec::DEFAULT
    }
}
