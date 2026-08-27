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

//! `IP::client_addr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::client_addr",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the client IP address of a connection.",
            synopsis: &["IP::client_addr"],
            snippet: "Returns the client IP address of a connection. This command is equivalent to the command clientside { IP::remote_addr } and to the BIG-IP 4.X variable client_addr.",
            source: "https://clouddocs.f5.com/api/irules/IP__client_addr.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [IP::addr [IP::client_addr] equals 10.10.10.10] } {\n     pool my_pool\n }\n}",
            return_value: "In BIG-IP 10.x with route domains enabled if the client is in any non-default route domain, this command returns the client IP address in the x.x.x.x%rd. For clients in the default route domain, it returns just the IPv4 address.",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["IP_GTM"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "IP::client_addr",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        xc_translatable: Some(true),
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::IP_ADDRESS)),
        ..CommandSpec::DEFAULT
    }
}
