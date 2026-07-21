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

//! `remote_addr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "remote_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deprecated: Use IP::remote_addr instead.",
            synopsis: &["remote_addr"],
            snippet: "Returns the IP address of the host on the far end of the connection. In the clientside context, this is the client IP address. In the serverside context this is the node IP address. You can also specify the IP::client_addr and IP::server_addr commands, respectively.\n\nIn BIG-IP 10.x with route domains enabled this command returns the remote IP address in the x.x.x.x%rd of the server or client (depending on the context) that is in any non-default route domain else it returns just the IP address as expected.\n\nThis command is equivalent to the BIG-IP 4.X variable remote_addr.",
            source: "https://clouddocs.f5.com/api/irules/remote_addr.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [IP::addr [IP::remote_addr] equals 206.0.0.0 mask 255.0.0.0] } {\n        pool clients_from_206\n    } else {\n        pool other_clients_pool\n    }\n}",
            return_value: "Returns the IP address of the host on the far end of the connection.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "remote_addr",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        deprecated_replacement: Some("IP::remote_addr"),
        deprecated_replacement_drop_in: true,
        ..CommandSpec::DEFAULT
    }
}
