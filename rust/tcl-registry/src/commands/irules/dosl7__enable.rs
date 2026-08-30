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

//! `DOSL7::enable` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::enable",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables blocking and detection of DoS attacks according to the ASM security policy configuration.",
            synopsis: &["DOSL7::enable (DOSL7_PROFILE_OBJ)?"],
            snippet: "Enables blocking and detection of DoS attacks according to the ASM\nsecurity policy configuration. When disabled using DOSL7::disable,\ntransactions will bypass DoS L7 for both detection and prevention.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__enable.html",
            examples: "when HTTP_REQUEST {\n    DOSL7::enable\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "DOSL7::enable (DOSL7_PROFILE_OBJ)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Dosl7State,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
