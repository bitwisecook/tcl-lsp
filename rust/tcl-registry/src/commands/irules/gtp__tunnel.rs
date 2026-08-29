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

//! `GTP::tunnel` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::tunnel",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "These commands parse the payload of G-PDU as IP datagram and return the values from IP header and TCP/UDP header.",
            synopsis: &["GTP::tunnel ('is_ip'"],
            snippet: "These commands parse the payload of G-PDU as IP datagram and return the\nvalues from IP header and TCP/UDP header.\nWhen parsed payload contains a value other than 4 or 6 for IP version,\nthe commands return an empty value. \"is_ip\" can be used to confirm if\nparser is considering the payload as ip-datagram or not. The commands\nreturn empty for non G-PDU messages.\ntcp_ and udp_ commands return empty value if the ip-proto in the\nip-datagram does not match. \"GTP::tunnel ip_proto\" may be used to\nverify before calling transport level commands.",
            source: "https://clouddocs.f5.com/api/irules/GTP__tunnel.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    log local0. \"GTP tunnel TCP src port [GTP::tunnel tcp_src_port]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "GTP::tunnel <subcommand> ?-message MESSAGE?",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on a specific GTP message object.",
                surface: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
