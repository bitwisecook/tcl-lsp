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

//! `TCP::lossfilterburst` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilterburst",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the TCP Loss Ignore Burst Parameter.",
            synopsis: &["TCP::lossfilterburst"],
            snippet: "Gets the maximum size burst loss (in packets) before triggering congestion response.\n  * Burst range is valid from 0 to 32. Higher values decrease the\n    chance of performing congestion control.",
            source: "https://clouddocs.f5.com/api/irules/TCP__lossfilterburst.html",
            examples: "when SERVER_CONNECTED {\n    # Set loss filter burst to a maximum of 3\n    if { [TCP::lossfilterburst] > 3 } {\n        TCP::lossfilter [TCP::lossfilterrate] 3\n    }\n}",
            return_value: "TCP Loss Ignore Burst in packets.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::lossfilterburst",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
