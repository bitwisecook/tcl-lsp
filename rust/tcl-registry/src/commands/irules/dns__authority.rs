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

//! `DNS::authority` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "clear",
        arity: Arity::exact(0),
        detail: "Clear all authority RRs.",
        synopsis: "DNS::authority clear",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::exact(1),
        detail: "Insert an RR into the authority section.",
        synopsis: "DNS::authority insert <rr_object>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::exact(1),
        detail: "Remove an RR from the authority section.",
        synopsis: "DNS::authority remove <rr_object>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::authority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns, inserts, removes, or clears RRs from the authority section.",
            synopsis: &["DNS::authority ('clear' | (('insert' | 'remove') RR_OBJECT))?"],
            snippet: "This iRules command returns, inserts, removes, or clears RRs from the\nauthority section.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__authority.html",
            examples: "authority record in all responses\n            when DNS_RESPONSE {\n                DNS::authority insert [DNS::rr \"devcentral.f5.com. 88 IN SOA 1.2.3.4\"]\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DNS::authority ?clear | insert <rr> | remove <rr>?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
