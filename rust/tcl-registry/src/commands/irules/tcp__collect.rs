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

//! `TCP::collect` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::collect",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(TCP_COLLECT),
        hover: Some(HoverSnippet {
            summary: "Collects the specified amount of data for delivery to the iRule.",
            synopsis: &["TCP::collect (COLLECT_BYTES (SKIP_BYTES)?)?"],
            snippet: "Collects the specified amount of data before triggering a CLIENT_DATA or SERVER_DATA event. This data is not delivered to the upper layer until the event fires.\n\nAfter collecting the data in a clientside event, the CLIENT_DATA\nevent will be triggered. When collecting the data in a serverside\nevent, the SERVER_DATA event will be triggered.\n\nIt is important to note that, when an explicit length is not specified,\nthe semantics of TCP::collect and TCP::release are different than\nthose of the HTTP::collect and HTTP::release commands.\n\n**Caution**: If multiple iRules call `TCP::collect` on the same\nconnection, only the last call takes effect (last-execution-wins).\nUse a marker variable to coordinate:\n```tcl\nset tcp_collect 0\nif { !$tcp_collect } {\n    set tcp_collect 1\n    TCP::collect 43\n}\n```",
            source: "https://clouddocs.f5.com/api/irules/TCP__collect.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # First collect 50000 bytes before passing data to the client.\n    TCP::collect 50000\n}",
            return_value: "None.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "TCP::collect (COLLECT_BYTES (SKIP_BYTES)?)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
