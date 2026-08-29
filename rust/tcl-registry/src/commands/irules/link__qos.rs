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

//! `LINK::qos` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::qos",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the QoS level set for the current packet.",
            synopsis: &["LINK::qos"],
            snippet: "Returns the QoS level set for the current packet.\nThe Quality of Service (QoS) standard is a means by which network\nequipment can identify and treat traffic differently based on an\nidentifier.\nThis command can be used to direct traffic based on the QoS level\nwithin a packet.\nThis command is equivalent to the BIG-IP 4.X variable link_qos.",
            source: "https://clouddocs.f5.com/api/irules/LINK__qos.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [LINK::qos] > 2 } {\n     pool fast_pool\n  } else {\n     pool slow_pool\n }\n}",
            return_value: "LINK::qos",
        }),
        forms: &[FormSpec {
            synopsis: "LINK::qos",
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
