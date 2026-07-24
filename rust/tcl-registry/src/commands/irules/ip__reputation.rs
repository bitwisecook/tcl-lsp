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

//! `IP::reputation` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::reputation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Looks up the supplied IP address in the IP intelligence (reputation) database and returns a TCL list containing reputation categories.",
            synopsis: &["IP::reputation (IP_ADDR)+"],
            snippet: "Performs a lookup of the supplied IP address against the IP reputation database. Returns a TCL list containing possible reputation categories:\n\nCategory                     Description\nBotnets                      IP addresses of computers that are infected with malicious software and are controlled as a group, and are now part of a botnet. Hackers can exploit botnets to send spam messages, launch various attacks, or cause target systems to behave in other unpredictable ways.\nCloud Provider Networks      IP addresses of cloud providers.",
            source: "https://clouddocs.f5.com/api/irules/IP__reputation.html",
            examples: "#Drop the packet after initial TCP handshake if the client has a bad reputation\nwhen CLIENT_ACCEPTED {\n    # Check if the IP reputation list for the client IP is not 0\n    if {[llength [IP::reputation [IP::client_addr]]] != 0}{\n        # Drop the connection\n        drop\n    }\n}",
            return_value: "Return a TCL list containing reputation categories.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IP::reputation (IP_ADDR)+",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
