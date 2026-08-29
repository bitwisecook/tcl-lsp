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

//! `MR::ignore_peer_port` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::ignore_peer_port",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the ignore_peer_port mode for the current connection.",
            synopsis: &["MR::ignore_peer_port (BOOLEAN)?"],
            snippet: "The MR::ignore_peer_port command sets or resets the ignore_peer_port mode of the current connection. If ignore_peer_port mode is enabled, the remote port of the connection will be ignored when determining if the connection is usable for forwarding a message to a peer. For example, if a peer at IP 10.1.2.3 connects using a ephemeral port of 12345 and ignore_peer_port is enabled, a message routed to IP 10.1.2.3 port 2345 can be forwarded using this connection since the port will be ignored.",
            source: "https://clouddocs.f5.com/api/irules/MR__ignore_peer_port.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::ignore_peer_port yes\n            }",
            return_value: "Returns the current value of the ignore_peer_port flag. This will be 'true' or 'false'.",
        }),
        forms: &[FormSpec {
            synopsis: "MR::ignore_peer_port (BOOLEAN)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
