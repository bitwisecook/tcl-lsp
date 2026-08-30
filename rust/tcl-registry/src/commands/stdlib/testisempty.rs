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

//! `testisempty` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testisempty",
        surface: Some(SpecSurface::TCL91),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Test whether a value is empty (Tcl 9.1).",
            synopsis: &["testisempty value"],
            snippet: "Returns whether ``value`` is the empty value via the internal ``Tcl_IsEmpty`` API.  Added in Tcl 9.1.",
            source: "Tcl test binary (tclTestObj.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
