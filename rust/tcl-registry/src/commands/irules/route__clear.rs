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

//! `ROUTE::clear` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::clear",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Removes a Congestion Metrics Cache entry.",
            synopsis: &["ROUTE::clear DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Removes the congestion metrics and MTU associated with a\ndestination IP address and/or gateway.\n\nClears the entry on all platform TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__clear.html",
            examples: "when CLIENT_ACCEPTED {\n    set bandwidth [ROUTE::bandwidth [IP::remote_addr]]\n    if { $bandwidth > 0 && $bandwidth < 1000 } {\n        # Reject cache entries below 1000 kbps\n        ROUTE::clear [IP::remote_addr]\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ROUTE::clear DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
