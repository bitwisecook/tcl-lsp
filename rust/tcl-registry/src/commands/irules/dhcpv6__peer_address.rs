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

//! `DHCPv6::peer_address` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::peer_address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns peer address field from DHCPv6 RELAY message.",
            synopsis: &["DHCPv6::peer_address"],
            snippet: "This command returns peer address field from DHCPv6 RELAY message\n\nDetails (syntax):\nDHCPv6::peer_address",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__peer_address.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Peer_address [DHCPv6::peer_address]\"\n    }",
            return_value: "This command returns peer address field from DHCPv6 RELAY message",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv6::peer_address",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
