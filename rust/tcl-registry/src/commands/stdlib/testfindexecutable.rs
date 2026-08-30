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

//! `testfindexecutable` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testfindexecutable",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Compute the executable path from an argv0.",
            synopsis: &["testfindexecutable argv0"],
            snippet: "Returns the path ``Tcl_FindExecutable`` would derive from ``argv0``, for testing executable-name resolution.",
            source: "Tcl test binary (tclUnixTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
