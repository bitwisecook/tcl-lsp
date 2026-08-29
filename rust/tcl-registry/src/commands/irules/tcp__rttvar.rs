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

//! `TCP::rttvar` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rttvar",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns TCP's smoothed RTT variance estimate.",
            synopsis: &["TCP::rttvar"],
            snippet: "Returns the Round Trip Time Variance, which is an indication of path jitter. TCP uses this figure, combined with RTT, to compute the RTO.\n\nNote that the value returned is in units of \"1/16 of a millisecond\". Divide the returned value by 16 to get the actual variance in milliseconds.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rttvar.html",
            examples: "when CLIENT_CLOSED {\n    # Log rttvar.\n    log local0. \"rttvar: [TCP::rttvar]\"\n}",
            return_value: "The measured RTT variance in units of \"1/16 of a millisecond\". Divide the returned value by 16 to get the actual variance in milliseconds.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::rttvar",
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
