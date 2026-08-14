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

//! `PSC::ip_address` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::ip_address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get/set/remove ip address(es).",
            synopsis: &[
                "PSC::ip_address (IP_ADDR)*",
                "PSC::ip_address 'add' IP_ADDR",
                "PSC::ip_address 'remove' (IP_ADDR)?",
            ],
            snippet: "The PSC::ip_address commands get/set/remove the IP addresses.\n\nNote:IP address used in the commands below could be in IPv4 or IPv6 format. The route domain can be specified using % as a separator, e.g. 14.15.16.17%10.",
            source: "https://clouddocs.f5.com/api/irules/PSC__ip_address.html",
            examples: "",
            return_value: "Return the list of PSC ip addresses when no argument is given.",
        }),
        forms: &[FormSpec {
            synopsis: "PSC::ip_address (IP_ADDR)*",
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
