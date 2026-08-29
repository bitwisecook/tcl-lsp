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

//! `LB::mode` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::mode",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the load balancing mode, overriding the mode set in the pool definition.",
            synopsis: &[
                "LB::mode (default | rr | roundrobin)",
                "LB::mode (leastconns | nodeleastconns)",
                "LB::mode (fastest)",
                "LB::mode (predictive)",
            ],
            snippet: "Sets the load balancing mode, overriding the mode set in the pool definition\n\nLB::mode [default | rr | roundrobin | leastconns |\n          fastest | predictive | observed | ratio |\n          dynratio | nodeleastconns | noderatio]",
            source: "https://clouddocs.f5.com/api/irules/LB__mode.html",
            examples: "when LB_SELECTED {\n    if { $myretry >= 1 } {\n        LB::mode rr\n        LB::reselect pool $mypool\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "LB::mode <mode>",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
