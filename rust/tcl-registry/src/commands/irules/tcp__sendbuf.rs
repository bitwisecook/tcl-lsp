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

//! `TCP::sendbuf` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::sendbuf",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the send buffer size of a TCP connection.",
            synopsis: &["TCP::sendbuf ('auto' | BUFFER_SIZE)?"],
            snippet: "TCP::sendbuf returns the send buffer size of a TCP connection.\nTCP::sendbuf BUFFER_SIZE sets the send buffer size to BUFFER_SIZE bytes.",
            source: "https://clouddocs.f5.com/api/irules/TCP__sendbuf.html",
            examples: "t the send buffer size of the TCP flow.\n    when CLIENT_ACCEPTED {\n        log local0. \"TCP set send buffer: [TCP::sendbuf 100000]\"\n        log local0. \"TCP get send buffer: [TCP::sendbuf]\"\n    }",
            return_value: "TCP::sendbuf returns the number of bytes that can be stored at the send buffer.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::sendbuf ('auto' | BUFFER_SIZE)?",
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
