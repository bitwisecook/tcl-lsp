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

//! `NSH::path_id` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::path_id",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set/Get the Path ID for NSH.",
            synopsis: &["NSH::path_id DIRECTION (NSH_PATH_ID)?"],
            snippet: "Set: Path ID for NSH.\n            Get(DIRECTION as the only parameter): path id from NSH.",
            source: "https://clouddocs.f5.com/api/irules/NSH__path_id.html",
            examples: "th ID for NSH.\n            when CLIENT_ACCEPTED {\n                NSH::path_id serverside_egress 10\n                set mypath_id [NSH::path_id serverside_egress]\n            }",
            return_value: "None for set, value of path id for get.",
        }),
        forms: &[FormSpec {
            synopsis: "NSH::path_id DIRECTION (NSH_PATH_ID)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
