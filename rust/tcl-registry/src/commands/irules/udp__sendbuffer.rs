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

//! `UDP::sendbuffer` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::sendbuffer",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the maximum send buffer size (bytes) of a UDP connection.",
            synopsis: &["UDP::sendbuffer (UDP_SNDBUF_SIZE)?"],
            snippet: "UDP::sendbuffer returns the maximum send buffer size (bytes) of a UDP connection.\nUDP::sendbuffer BUFFERSIZE sets the maximum send buffer size (bytes) to specified value.",
            source: "https://clouddocs.f5.com/api/irules/UDP__sendbuffer.html",
            examples: "# Get/set the send buffer size of the UDP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"UDP get send buffer: [UDP::sendbuffer]\"\n    # Set the send buffer to 2,000,000 bytes\n    log local0. \"UDP set send buffer: [UDP::sendbuffer 2000000]\"\n    log local0. \"UDP get send buffer: [UDP::sendbuffer]\"\n}",
            return_value: "UDP::sendbuffer returns the maximum send buffer size (bytes) of a UDP connection.",
        }),
        forms: &[FormSpec {
            synopsis: "UDP::sendbuffer (UDP_SNDBUF_SIZE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::UdpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
