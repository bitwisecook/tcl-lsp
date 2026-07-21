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

//! `MR::always_match_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::always_match_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the always_match_port mode for the router.",
            synopsis: &["MR::always_match_port (BOOLEAN)?"],
            snippet: "The MR::always_match_port command sets or resets the always_match_port mode of the current router. If always_match_port mode is enabled (upon completion of CLIENT_ACCEPTED event), the router will only forward messages to existing connections where the remote port matches the remote port of the selected destination. If an existing connection is not found, a new connection will be created. Setting this mode will keep MRF from forwarding messages to incoming connections (since the incoming connection likely uses a ephemeral port as the source port).",
            source: "https://clouddocs.f5.com/api/irules/MR__always_match_port.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::always_match_port no\n            }",
            return_value: "Returns the current value of the always_match_port flag. This will be 'true' or 'false'.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MR::always_match_port (BOOLEAN)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
