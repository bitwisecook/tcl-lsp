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

//! `MR::prime` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::prime",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "establishes an outgoing connection to the specified host or hosts using the specified transport",
            synopsis: &[
                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
            ],
            snippet: "The MR::prime command instructs the Message Routing Framework to establish an outgoing connection to a specified host or pool if one does not exist. The setting of the specified virtual or transport-config will be used to establish the connection. If a pool is provided, outgoing connections will be created to all active poolmembers of the specified pool.",
            source: "https://clouddocs.f5.com/api/irules/MR__prime.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::prime config /Common/my_tc pool /Common/default_pool\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
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
