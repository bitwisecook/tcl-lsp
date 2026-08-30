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

//! `teststatproc` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "teststatproc",
        surface: Some(SpecSurface::TCL84),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Install or remove a test stat(2) hook proc (Tcl 8.4 only).",
            synopsis: &["teststatproc option arg"],
            snippet: "``option`` is ``insert`` or ``delete``; ``arg`` names the proc (``TestStatProc1``, ``TestStatProc2``, ``TestStatProc3``, or ``TclpStat``).  Removed in Tcl 8.5 (obsolete FS hooks compiled out).",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
