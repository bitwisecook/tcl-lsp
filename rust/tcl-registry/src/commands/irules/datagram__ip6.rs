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

//! `DATAGRAM::ip6` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::ip6",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns ipv6 header information.",
            synopsis: &[
                "DATAGRAM::ip6 hop_limit",
                "DATAGRAM::ip6 (option | option_count) (IPV6_OPTION)?",
            ],
            snippet: "This iRules command returns ipv6 header information.\nNote: throws an error when used with IPv4\n\nDATAGRAM::ip6 hop_limit\n\n     * This command returns IPv6 hop limit as an integer value.\n\nDATAGRAM::ip6 option\n\n     * This command returns a Tcl list of IPv6 options from reassembled\n       IPv6 datagram. Each option is a Tcl list with one or two values -\n       option code (integer), and option value (byte array) if option has\n       the value. Multiple options with the same code will be returned as\n       separate sublists.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__ip6.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &["CLIENT_DATA"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DATAGRAM::ip6 hop_limit",
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
