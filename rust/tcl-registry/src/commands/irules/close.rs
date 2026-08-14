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

//! `close` iRules command.
//!
//! iRules' `close` closes a sideband connection — the first
//! positional argument plays the same channel-id role as Tcl's
//! `close channelId`. Setting `arg_roles` here keeps the
//! channel-type diagnostic working when the iRules dialect is
//! loaded (otherwise the iRules override shadows the Tcl spec
//! with empty roles and the diagnostic silently stops firing).
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel)],
        hover: Some(HoverSnippet {
            summary: "Closes an existing sideband connection.",
            synopsis: &["close CONNECTION"],
            snippet: "This command closes an existing sideband connection. It is one of several commands that make up the ability to create sideband connections from iRules.",
            source: "https://clouddocs.f5.com/api/irules/close.html",
            examples: "# Open a sideband connection with a connection timeout of 100 ms and an idle timeout of 30 seconds\n#   to a local virtual server name sideband_virtual_server\nset conn_id [connect -timeout 100 -idle 30 -status conn_status sideband_virtual_server]\n\n# Same as above, but use an external host IP:port instead of a virtual server name\nset conn_id [connect -timeout 100 -idle 30 -status conn_status 10.0.0.10:80]\n\n# close the connection\nclose conn_id",
            return_value: "close <connection> closes an existing connection",
        }),
        forms: &[FormSpec {
            synopsis: "close CONNECTION",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
