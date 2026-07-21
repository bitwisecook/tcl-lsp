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

//! `TCP::setmss` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::setmss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the TCP max segment size.",
            synopsis: &["TCP::setmss TCP_MAX_SEGMENT_SIZE"],
            snippet: "This iRule command sets the TCP max segment size in bytes.\nThe MSS does not consider the length of any common TCP options.\nUsers should set MSS to the desired path IP packet size, minus the\nIP header length (typically 20 bytes), minus the minimum TCP header\nlength of 20 bytes.\n\nTCP will automatically apply the length of common options when\npartitioning data for delivery.",
            source: "https://clouddocs.f5.com/api/irules/TCP__setmss.html",
            examples: "# Match clientside MSS to serverside MSS\nwhen SERVER_CONNECTED {\n    set cli_mss [clientside { TCP::mss }]\n    set svr_mss [TCP::mss]\n    if { $cli_mss > $svr_mss } {\n        clientside { TCP::setmss $svr_mss }\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::setmss TCP_MAX_SEGMENT_SIZE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
