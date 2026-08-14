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

//! `RESOLVER::name_lookup` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLVER::name_lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command performs a DNS lookup.",
            synopsis: &["RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE"],
            snippet: "RESOLVER::name_lookup performs a DNS query for a name and type using the network resolver specified.\n\nThis command allows queries for all resource record types and provides access to the DNS services available on the BIGIP.",
            source: "https://clouddocs.f5.com/api/irules/RESOLV-lookup.html",
            examples: "when CLIENT_ACCEPTED {\n        set result [RESOLVER::name_lookup \"/Common/r1\" www.abc.com a]\n        set result [RESOLVER::name_lookup \"/Common/r1\" 206.6.177.2 ptr]\n}",
            return_value: "The response will be a dns_message structure usable by the RESOLVER::summarize, DNSMSG::header, and DNSMSG::section iRule commands.",
        }),
        forms: &[FormSpec {
            synopsis: "RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
