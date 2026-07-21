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

//! `ROUTE::mtu` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::mtu",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the cached MTU entry.",
            synopsis: &["ROUTE::mtu DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the cached MTU entry for the provided destination and/or gateway.\n\nUnlike other ROUTE::commands, this value is valid across all TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__mtu.html",
            examples: "when CLIENT_ACCEPTED {\n    set mtu [ROUTE::mtu [IP::remote_addr]]\n    if { $mtu > 0 && $mtu < 300 } {\n        #Ignore extremely small cached MTUs\n        ROUTE::clear [IP::remote_addr]\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ROUTE::mtu DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
