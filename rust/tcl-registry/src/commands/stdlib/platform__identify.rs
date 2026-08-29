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

//! `platform::identify` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::identify",
        traits: Traits::PURE,
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Return the platform identifier for the current machine.",
            synopsis: &["platform::identify"],
            snippet: "Returns a string like ``linux-x86_64`` or ``macosx-arm`` that specifically identifies the current platform, including CPU details and libc version where relevant.",
            source: "Tcl stdlib platform package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("platform"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
