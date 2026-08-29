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

//! `MR::equivalent_transport` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::equivalent_transport",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the transport that is usable as an equivalent transport.",
            synopsis: &[
                "MR::equivalent_transport",
                "MR::equivalent_transport none",
                "MR::equivalent_transport (('virtual' VIRTUAL_SERVER_OBJ) | ('config' TRANSPORT_CONFIG))",
            ],
            snippet: "Gets or sets the transport that is usable as an equivalent transport. The equivalent transport may be used as an alternate when selecting a subsequent connection to the device the current connections communicates with.\n        \nGets the transport that is usable as an equivalent transport. The equivalent transport may be used as an alternate when selecting a subsequent connection to the device the current connections communicates with.\n            \nResets the transport that is usable as an equivalent transport.",
            source: "https://clouddocs.f5.com/api/irules/MR__equivalent_transport.html",
            examples: "when CLIENT_ACCEPTED {\n    MR::equivalent_transport config /Common/inbound_tc\n}",
            return_value: "Returns the current equivalent transport. This will contain the transport type and transport name. For example: 'config /Common/inbound_tc'.",
        }),
        forms: &[FormSpec {
            synopsis: "MR::equivalent_transport",
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
