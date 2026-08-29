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

//! `DNS::header` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "cd",
        surface: None,
        arity: Arity::new(0, 1),
        detail: "Get/set the cd header field.",
        synopsis: "DNS::header cd ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "aa",
        arity: Arity::new(0, 1),
        detail: "Get/set the aa header field.",
        synopsis: "DNS::header aa ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ra",
        arity: Arity::new(0, 1),
        detail: "Get/set the ra header field.",
        synopsis: "DNS::header ra ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rd",
        arity: Arity::new(0, 1),
        detail: "Get/set the rd header field.",
        synopsis: "DNS::header rd ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "arcount",
        arity: Arity::new(0, 1),
        detail: "Get/set the arcount header field.",
        synopsis: "DNS::header arcount ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "qdcount",
        arity: Arity::new(0, 1),
        detail: "Get/set the qdcount header field.",
        synopsis: "DNS::header qdcount ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nscount",
        arity: Arity::new(0, 1),
        detail: "Get/set the nscount header field.",
        synopsis: "DNS::header nscount ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "id",
        arity: Arity::new(0, 1),
        detail: "Get/set the id header field.",
        synopsis: "DNS::header id ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tc",
        arity: Arity::new(0, 1),
        detail: "Get/set the tc header field.",
        synopsis: "DNS::header tc ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "qr",
        arity: Arity::new(0, 1),
        detail: "Get/set the qr header field.",
        synopsis: "DNS::header qr ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ancount",
        arity: Arity::new(0, 1),
        detail: "Get/set the ancount header field.",
        synopsis: "DNS::header ancount ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "opcode",
        arity: Arity::new(0, 1),
        detail: "Get/set the opcode header field.",
        synopsis: "DNS::header opcode ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rcode",
        arity: Arity::new(0, 1),
        detail: "Get/set the rcode header field.",
        synopsis: "DNS::header rcode ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ad",
        arity: Arity::new(0, 1),
        detail: "Get/set the ad header field.",
        synopsis: "DNS::header ad ?value?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
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
        name: "DNS::header",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets (v11.0+) or sets (v11.1+) simple bits or byte fields.",
            synopsis: &[
                "DNS::header <field> ?value?",
                "DNS::header id ?UNSIGNED_SHORT?",
                "DNS::header rcode ?RCODE_VALUE?",
            ],
            snippet: "This iRules command gets or sets simple bits or byte fields. Read-only\nform introduced in v11.0, Read-write capability added in v11.1.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__header.html",
            examples: "queries from a specific ip\n            when DNS_REQUEST {\n                if { [IP::client_addr] equals \"192.168.1.245\" } {\n                    DNS::answer clear\n                    DNS::header rcode REFUSED\n                    DNS::return\n                    return\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DNS::header <field> ?value?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
