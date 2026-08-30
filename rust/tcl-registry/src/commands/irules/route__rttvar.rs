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

//! `ROUTE::rttvar` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::rttvar",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the cached round-trip-time variance (rttvar) estimate.",
            synopsis: &["ROUTE::rttvar DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the cached round-trip-time variance (rttvar) for the\ndestination and/or gateway if the relevant TCP profile enables\ncmetrics-cache.\n\nThe return value only applies to the TMM executing the command. It\ndoes not consider cache entries on other TMMs.\n\nROUTE::rttvar returns a value of 0 when there are no statistics\navailable.\n\nNOTE: The returned value is scaled to units of 100ns; to express it\nin the same units as TCP::rttvar multiply it by 16/10000.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__rttvar.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"Cached rttvar is: [ROUTE::rttvar [IP::remote_addr]]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ROUTE::rttvar DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
