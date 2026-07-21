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

//! `RESOLV::lookup` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLV::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deprecated: The commands for making a DNS lookup.",
            synopsis: &["RESOLV::lookup"],
            snippet: "RESOLV::lookup performs a DNS query, returning one or more addresses (A records) for a hostname, a domain name (PTR record) for an IP address, or optionally one or more values for records of other types.",
            source: "https://clouddocs.f5.com/api/irules/RESOLV__lookup.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "RESOLV::lookup ?@nameserver? ?-type? hostname",
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-a",
                    value: OptionValue::flag(),
                    detail: "Query for type A (IPv4) records.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-aaaa",
                    value: OptionValue::flag(),
                    detail: "Query for type AAAA (IPv6) records.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-ptr",
                    value: OptionValue::flag(),
                    detail: "Query for PTR records.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-txt",
                    value: OptionValue::flag(),
                    detail: "Query for TXT records.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-mx",
                    value: OptionValue::flag(),
                    detail: "Query for MX records.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
