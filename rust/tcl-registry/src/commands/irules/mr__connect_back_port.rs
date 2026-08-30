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

//! `MR::connect_back_port` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connect_back_port",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets connect_back_port for the current connection.",
            synopsis: &["MR::connect_back_port (NONNEGATIVE_INTEGER)?"],
            snippet: "The MR::connect_back_port command gets or sets connect_back_port for the current connection, which is used to enable bi-directional persistence if the client connected through an ephemeral port.",
            source: "https://clouddocs.f5.com/api/irules/MR__connect_back_port.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::connect_back_port 5678\n            }",
            return_value: "Returns current connect_back_port value.",
        }),
        forms: &[FormSpec {
            synopsis: "MR::connect_back_port (NONNEGATIVE_INTEGER)?",
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
