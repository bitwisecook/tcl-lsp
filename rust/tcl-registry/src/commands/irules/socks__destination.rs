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

//! `SOCKS::destination` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SOCKS::destination",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command allows you to get or set the SOCKS destination host or port.",
            synopsis: &[
                "SOCKS::destination ('host')? (HOST_ADDRESS)?",
                "SOCKS::destination 'port' (PORT)?",
            ],
            snippet: "This command allows you to get or set the SOCKS host or port, individually, or both at the same time.\n\nDetails (Syntax):\nSOCKS::destination \"hostname:port\"\n    Sets the destination to the given hostname and port tuple.\n\nSOCKS::destination\n    Gets the destination in the format \"hostname:port\".\n\nSOCKS::destination host \"hostname\"\n    Sets the destination to the given hostname, doesn't change the port.\nSOCKS::destination host\n    Gets the destination hostname.  (Without appending the port.)\n\nSOCKS::destination port \"port_number\"\n    Sets the destination port, doesn't change the hostname.",
            source: "https://clouddocs.f5.com/api/irules/SOCKS__destination.html",
            examples: "when SOCKS_REQUEST {\n    SOCKS::destination example.com:1234\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SOCKS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "SOCKS::destination ('host')? (HOST_ADDRESS)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
