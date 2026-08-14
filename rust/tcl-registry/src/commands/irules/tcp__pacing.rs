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

//! `TCP::pacing` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::pacing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles TCP rate pacing.",
            synopsis: &["TCP::pacing (BOOL_VALUE)?"],
            snippet: "Rate pacing limits the data send rate to the physical limitations of the interface to reduce the chance of queue drops.",
            source: "https://clouddocs.f5.com/api/irules/TCP__pacing.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # Set client-side rate pacing to enabled.\n    clientside {\n        log local0. \"Client: pacing [TCP::pacing], enabling\"\n        TCP::pacing enable\n    }\n    # Set server-side rate pacing to disabled.\n    serverside {\n        log local0. \"Server: pacing [TCP::pacing], disabling\"\n        TCP::pacing disable\n    }\n}",
            return_value: "TCP::pacing returns whether TCP rate pacing is enabled.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::pacing (BOOL_VALUE)?",
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
