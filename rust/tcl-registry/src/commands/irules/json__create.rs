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

//! `JSON::create` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::create",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a new, empty JSON cache instance.",
            synopsis: &["JSON::create"],
            snippet: "Creates a new, empty JSON cache instance. It can then be filled with any JSON content and rendered. It will be deleted when no longer referenced by a Tcl variable.",
            source: "https://clouddocs.f5.com/api/irules/JSON__create.html",
            examples: "when JSON_REQUEST {\n    set cache [JSON::create]\n    set rootval [JSON::root $cache]\n    JSON::set $rootval string HelloWorld\n    set rendered [JSON::render $cache]\n}",
            return_value: "Returns the new JSON cache instance.",
        }),
        forms: &[FormSpec {
            synopsis: "JSON::create",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
