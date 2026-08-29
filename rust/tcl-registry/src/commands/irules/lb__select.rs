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

//! `LB::select` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::select",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Forces a load balancing selection and returns the result.",
            synopsis: &["LB::select"],
            snippet: "This command forces the system to make a load balancing selection based on current conditions, and returns a string in the form of a pool command that can be eval'd to activate that selection.",
            source: "https://clouddocs.f5.com/api/irules/LB__select.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "LB::select",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            writes: true,
            connection_side: ConnectionSide::Server,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
