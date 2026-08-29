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

//! `IP::addr` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::addr",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "IP address comparison.",
            synopsis: &[
                "IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK",
                "IP::addr 'parse' ((('-swap')? BINARY_FIELD (OFFSET)?) |",
            ],
            snippet: "IP address comparison\n\nPerforms comparison of IP address/subnet/supernet to IP address/subnet/supernet.\n\nReturns 0 if no match, 1 for a match.\n\nUse of IP::addr is not necessary if the class (v10+) or matchclass (v9) command is used to perform the address-to-address comparison.\n\nDoes NOT perform a string comparison. To perform a literal string comparison, simply compare the 2 strings with the appropriate operator (equals, contains, starts_with, etc) rather than using the IP::addr comparison.\n\nFor versions 10.0 - 10.2.",
            source: "https://clouddocs.f5.com/api/irules/IP__addr.html",
            examples: "# To select a specific pool for a specific client IP address.\nwhen CLIENT_ACCEPTED {\n   if { [IP::addr [IP::client_addr] equals 10.10.10.10] } {\n      pool my_pool\n   }\n}",
            return_value: "Returns 0 IF NO MATCH, 1 for a match.",
        }),
        forms: &[FormSpec {
            synopsis: "IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-swap",
                    value: OptionValue::flag(),
                    detail: "Swap byte order.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-ipv4",
                    value: OptionValue::flag(),
                    detail: "Parse as IPv4 address.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-ipv6",
                    value: OptionValue::flag(),
                    detail: "Parse as IPv6 address.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
