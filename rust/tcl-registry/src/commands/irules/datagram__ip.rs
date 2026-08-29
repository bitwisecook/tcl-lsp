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

//! `DATAGRAM::ip` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::ip",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns ip header information.",
            synopsis: &[
                "DATAGRAM::ip (tos | ttl | flags)",
                "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?",
            ],
            snippet: "This iRules command returns ip header information.\n\nDATAGRAM::ip tos\n\n     * Returns IP header ToS as an integer value.\n\nDATAGRAM::ip ttl\n\n     * Returns IP header TTL as an integer value.\n\nDATAGRAM::ip flags\n\n     * Returns IP header flags as an integer value. The flags are from the\n       IP datagram after IP fragment reassembly. Any MF flags that were\n       present in indivdual fragments will not be returned. DF flag is\n       preserved if it was set.\n\nDATAGRAM::ip option\n\n     * This command returns a Tcl list of IP options from reassembled IP\n       datagram.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__ip.html",
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
            synopsis: "DATAGRAM::ip (tos | ttl | flags)",
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
