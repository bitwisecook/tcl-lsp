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

//! `TCP::bandwidth` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::bandwidth",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the estimated bandwidth of the connection.",
            synopsis: &["TCP::bandwidth"],
            snippet: "Returns the estimated bandwidth measured as \"congestion window\" / \"the measured round trip time\".\nThe values returned are only estimates, and can vary even during the connection.\n\nNote: Starting with BIG-IP v9.4.2, client bandwidth calculations are unavailable, always returning 0. Starting with BIG-IP v12.0 nonzero values are returned.",
            source: "https://clouddocs.f5.com/api/irules/TCP__bandwidth.html",
            examples: "when CLIENT_DATA {\nif {[HTTP::uri] starts_with \"/xxx.css\"}{\n      set bandwidth [TCP::bandwidth]\n      if {$bandwidth < XXX} {\n         HTTP::uri \"/boring-xxx.css\"\n      }\n   }\n}",
            return_value: "The estimated bandwidth in kilobits per second.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "TCP::bandwidth",
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
