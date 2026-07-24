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

//! `UDP::debug_queue` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::debug_queue",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to enable/disable printing debug messages when UDP::max_rate iRule is in use.",
            synopsis: &["UDP::debug_queue BOOL_VALUE"],
            snippet: "UDP::debug_queue enable starts printing debug messages related to UDP::max_rate.\nUDP::debug_queue disable stops printing debug messages related to UDP::max_rate.",
            source: "https://clouddocs.f5.com/api/irules/UDP__debug_queue.html",
            examples: "when SERVER_CONNECTED {\n    # Set the rate to 1Mbps (125,000 bytes per second)\n    log local0. \"UDP set max rate: [UDP::max_rate 125000]\"\n    log local0. \"UDP get max rate: [UDP::max_rate]\"\n    # Enable printing debug messages.\n    log local0. \"Enable debugging [UDP::debug_queue enable]\"\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "UDP::debug_queue BOOL_VALUE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::UdpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
