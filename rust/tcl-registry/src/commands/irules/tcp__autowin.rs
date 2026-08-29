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

//! `TCP::autowin` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::autowin",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles automatic window tuning.",
            synopsis: &["TCP::autowin BOOL_VALUE"],
            snippet: "Sets the send and receive buffer dynamically in accordance with measured connection parameters.",
            source: "https://clouddocs.f5.com/api/irules/TCP__autowin.html",
            examples: "when HTTP_REQUEST {\n    # Enable auto buffer tuning on HTTP request(s).\n    log local0. \"Send buffer: [TCP::sendbuf] Receive Window: [TCP::recvwnd]\"\n    log local0. \"HTTP request, auto buffer tuning enabled.\"\n    TCP::autowin enable\n    log local0. \"Send buffer: [TCP::sendbuf] Receive Window: [TCP::recvwnd]\"\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::autowin BOOL_VALUE",
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
