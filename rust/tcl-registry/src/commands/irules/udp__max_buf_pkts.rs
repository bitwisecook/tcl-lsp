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

//! `UDP::max_buf_pkts` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::max_buf_pkts",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the maximum buffer packets value of a UDP connection.",
            synopsis: &["UDP::max_buf_pkts (UDP_MAX_BUF_PKTS)?"],
            snippet: "UDP::max_buf_pkts returns the maximum buffer packets value of a UDP connection.\nUDP::max_buf_pkts UDP_MAX_BUF_PKTS sets the maximum buffer packets value to specified value.",
            source: "https://clouddocs.f5.com/api/irules/UDP__max_buf_pkts.html",
            examples: "# Get/set the max buffer packets of the UDP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"UDP get max buffer packets: [UDP::max_buf_pkts]\"\n    # Set the max buffer packets to 5,000\n    log local0. \"UDP set max buffer packets: [UPD::max_buf_pkts 5000]\"\n    log local0. \"UDP get max buffer packets: [UDP::max_buf_pkts]\"\n}",
            return_value: "UDP::max_buf_pkts returns the maximum buffer packets value of a UDP connection.",
        }),
        forms: &[FormSpec {
            synopsis: "UDP::max_buf_pkts (UDP_MAX_BUF_PKTS)?",
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
