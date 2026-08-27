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

//! `TCP::unused_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::unused_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns an unused TCP port for the specified IP tuple.",
            synopsis: &["TCP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR"],
            snippet: "Returns an unused TCP port for the specified IP tuple, using the value\nof <hint_port> as a starting point if it is supplied. If no appropriate\nunused local port could be found, 0 is returned.",
            source: "https://clouddocs.f5.com/api/irules/TCP__unused_port.html",
            examples: "when HTTP_REQUEST {\n    log local0. [TCP::unused_port 192.168.1.124 80 192.168.1.146]\n    }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "TCP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR",
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
