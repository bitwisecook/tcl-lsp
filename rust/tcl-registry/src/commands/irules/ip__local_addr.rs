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

//! `IP::local_addr` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::local_addr",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the IP address of the virtual server the client is connected to or the self-ip LTM is connected from.",
            synopsis: &["IP::local_addr (clientside | serverside)?"],
            snippet: "When called in a clientside context, this command returns the IP address of the virtual server the client is connected to. When called in a serverside context it returns the self-ip address or spoofed client IP address LTM is using for the serverside connection.\n\nThis command is primarily useful for generic rules that are re-used. Also, it is useful in reusing the connected endpoint in another statement (such as with the listen command) or to make routing type decisions. You can also specify the IP::client_addr and IP::server_addr commands.\n\nThis command in BIG-IP 10.",
            source: "https://clouddocs.f5.com/api/irules/IP__local_addr.html",
            examples: "when SERVER_CONNECTED {\n   log local0. \"Source IP address for connection to node: [IP::local_addr]\"\n}",
            return_value: "Returns the IP address being used in the connection.",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["IP_GTM"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "IP::local_addr (clientside | serverside)?",
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
