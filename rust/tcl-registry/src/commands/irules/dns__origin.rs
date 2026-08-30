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

//! `DNS::origin` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::origin",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the originator of the DNS message.",
            synopsis: &["DNS::origin"],
            snippet: "Returns the last module to modify the DNS message. Return values:\n\nCLIENT\n\n   This message has just been received by the BigIP from a client's query,\n   and nothing has been processed.\n\nSERVER\n\n   This message has just been received by the BigIP from a server's\n   response to a DNS query, such as On-Box or Off-Box BIND, or another\n   BigIP entirely.\n\nCACHE\n\n   This message is a response from the DNS Cache.\n\nRPZ\n\n   This message is a response from the Response Policy Zone in your BigIP.\n   It was blocked and either NXDOMAIN or a Walled Garden was returned as a\n   response.",
            source: "https://clouddocs.f5.com/api/irules/DNS__origin.html",
            examples: "equests that were not resolved by DNS Express\n            when DNS_RESPONSE {\n                if { [DNS::origin] ne \"DNSX\" } {\n                    DNS::drop\n                }\n            }",
            return_value: "CLIENT SERVER CACHE GTM_BUILD GTM_REWRITE DNSX DNSSEC LAST_ACTION TCL RPZ RATE_LIMITER",
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
            synopsis: "DNS::origin",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
