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

//! `SIP::via` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "proto",
        arity: Arity::new(0, 1),
        detail: "Get Via protocol.",
        synopsis: "SIP::via proto ?index?",
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
        name: "sent_by",
        arity: Arity::new(0, 1),
        detail: "Get Via sent-by field.",
        synopsis: "SIP::via sent_by ?index?",
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
        name: "received",
        arity: Arity::new(0, 1),
        detail: "Get Via received parameter.",
        synopsis: "SIP::via received ?index?",
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
        name: "branch",
        arity: Arity::new(0, 1),
        detail: "Get Via branch parameter.",
        synopsis: "SIP::via branch ?index?",
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
        name: "maddr",
        arity: Arity::new(0, 1),
        detail: "Get Via maddr parameter.",
        synopsis: "SIP::via maddr ?index?",
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
        name: "ttl",
        arity: Arity::new(0, 1),
        detail: "Get Via ttl parameter.",
        synopsis: "SIP::via ttl ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::via",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets SIP via header information.",
            synopsis: &[
                "SIP::via ?field? ?INDEX?",
                "SIP::via (proto | sent_by | received | branch | maddr | ttl) ?INDEX?",
            ],
            snippet: "This set of commands allows you to get information in the SIP via header.",
            source: "https://clouddocs.f5.com/api/irules/SIP__via.html",
            examples: "when SIP_RESPONSE {\n  log local0. [SIP::via 0]\n  SIP::header remove Via 0\n  SIP::response rewrite 123 \"no xxx\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "SIP::via ?field? ?INDEX?",
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
