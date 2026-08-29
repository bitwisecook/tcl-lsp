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

//! `pkg::create` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg::create",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::at_least(4),
        hover: Some(HoverSnippet {
            summary: "Generate a ``package ifneeded`` script for a package.",
            synopsis: &[
                "pkg::create -name packageName -version packageVersion ?-load filespec? ?-source filespec?",
            ],
            snippet: "",
            source: "Tcl stdlib package utilities",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
