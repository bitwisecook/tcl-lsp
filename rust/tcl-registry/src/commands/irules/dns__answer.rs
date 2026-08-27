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

//! `DNS::answer` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "clear",
        arity: Arity::exact(0),
        detail: "Clear all answer RRs.",
        synopsis: "DNS::answer clear",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::exact(1),
        detail: "Insert an RR into the answer section.",
        synopsis: "DNS::answer insert <rr_object>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::exact(1),
        detail: "Remove an RR from the answer section.",
        synopsis: "DNS::answer remove <rr_object>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::answer",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns, inserts, removes, or clears all RRs from the answer section.",
            synopsis: &["DNS::answer ('clear' | (('insert' | 'remove') RR_OBJECT))?"],
            snippet: "This iRules command returns, inserts, removes, or clears RRs from the\nanswer section.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__answer.html",
            examples: "ttl of all answer records and add a glue record\n            when DNS_RESPONSE {\n                set rrs [DNS::answer]\n                foreach rr $rrs {\n                    DNS::ttl $rr 1234\n                }\n                set new_rr [DNS::rr \"bigip3900-30.f5net.com. 88 IN A 1.2.3.4\"]\n                DNS::additional insert $new_rr\n            }",
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
            synopsis: "DNS::answer ?clear | insert <rr> | remove <rr>?",
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
