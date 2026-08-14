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

//! `TCP::rcv_scale` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rcv_scale",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the receive window scale advertised by the remote host.",
            synopsis: &["TCP::rcv_scale"],
            snippet: "Returns the receive window scale advertised by the remote host.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rcv_scale.html",
            examples: "when CLIENT_ACCEPTED {\n    # Log rcv_scale.\n    log local0. \"rcv_scale: [TCP::rcv_scale]\"\n}",
            return_value: "The bitshift associated with the remote host window scale.",
        }),
        excluded_events: &["SERVER_INIT"],
        forms: &[FormSpec {
            synopsis: "TCP::rcv_scale",
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
