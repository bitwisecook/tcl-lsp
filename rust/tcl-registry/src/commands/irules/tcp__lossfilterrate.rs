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

//! `TCP::lossfilterrate` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilterrate",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the TCP Loss Ignore Rate Parameter.",
            synopsis: &["TCP::lossfilterrate"],
            snippet: "Gets the maximum number of packets per million lost before triggering congestion response.\n  * Rate range is valid from 0 to 1,000,000. Rate is X packets lost per\n    million before congestion control kicks in.",
            source: "https://clouddocs.f5.com/api/irules/TCP__lossfilterrate.html",
            examples: "when SERVER_CONNECTED {\n    # Remove loss filter if present\n    if { [TCP::lossfilterrate] > 0 } {\n        TCP::lossfilter 0 0\n    }\n}",
            return_value: "TCP Loss Ignore Rate in packets per million.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::lossfilterrate",
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
