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

//! `TCP::snd_wnd` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_wnd",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "The remote host's advertised receive window.",
            synopsis: &["TCP::snd_wnd"],
            snippet: "Returns the remote host's advertised receive window. If smaller\nthan the congestion window (cwnd) and send buffer size, this limits\nthe amount of outstanding data on the connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__snd_wnd.html",
            examples: "when CLIENT_CLOSED {\n    # Get Client's last advertised window.\n    log local0. \"Client's advertised rwnd: [TCP::snd_wnd]\"\n}",
            return_value: "The advertised receive window (rwnd) in bytes.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::snd_wnd",
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
