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

//! `ROUTE::bandwidth` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::bandwidth",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a bandwidth estimate for a destination derived from entries in the congestion metrics cache.",
            synopsis: &["ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns a bandwidth estimate for a destination derived from\nentries in the congestion metrics cache.\n\nAs of v12.0, divides the cached congestion window (cwnd) value\nby the cached round-trip-time (RTT ) to obtain a bandwidth\nestimate in kbps. If there is no entry, it returns 0.\n\nNote: The return value only applies to the TMM executing the command.\nIt does not consider cache entries on other TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__bandwidth.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [ROUTE::bandwidth [IP::remote_addr]] > 0 } {\n        log local0. \"cached bandwidth is: [ROUTE::bandwidth [IP::remote_addr]]\"\n    }\n}",
            return_value: "The bandwidth estimate to the destination and/or gateway in kbps.",
        }),
        forms: &[FormSpec {
            synopsis: "ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
