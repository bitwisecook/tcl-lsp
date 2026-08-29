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

//! `tmsh::clear_screen` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "tmsh::clear_screen",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::clear_screen",
        surface: Some(surface![SpecSurface::package("iapps"), SpecSurface::package("tmsh")]),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Clears the screen.",
            &["tmsh::clear_screen"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
