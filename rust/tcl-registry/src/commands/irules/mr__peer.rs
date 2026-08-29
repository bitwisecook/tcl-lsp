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

//! `MR::peer` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::peer",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Defines a peer to use for routing a message to.",
            synopsis: &[
                "MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))",
            ],
            snippet: "The MR::peer command defines a peer to use for routing a message to. The peer may either refer to a named pool or a tuple (IP address, port and route domain iD). When creating a connection to a peer, the parameters of either a virtual server or a transport config object will be used. The peer object will only exist in the current connections connflow. When adding a route (via MR::route add), it will first look for a locally created peer object then for a peer object from the configuration. Once the current connection closes, the local peer object will go away.",
            source: "https://clouddocs.f5.com/api/irules/MR__peer.html",
            examples: "when CLIENT_ACCEPTED {\n    MR::peer self_peer config tc1 host \"[IP::remote_addr]:[TCP::remote_port]\"\n    GENERICMESSAGE::route add dest \"[IP::remote_addr]\" peer self_peer\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))",
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
