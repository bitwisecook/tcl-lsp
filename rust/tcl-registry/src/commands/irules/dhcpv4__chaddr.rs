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

//! `DHCPv4::chaddr` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::chaddr",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns chaddr (client hardware address) from DHCPv4 message.",
            synopsis: &["DHCPv4::chaddr"],
            snippet: "This command returns chaddr (client hardware address) from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::chaddr",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__chaddr.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Chaddr [DHCPv4::chaddr]\"\n    }",
            return_value: "This command returns chaddr (client hardware address) from DHCPv4 message",
        }),
        forms: &[FormSpec {
            synopsis: "DHCPv4::chaddr",
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
