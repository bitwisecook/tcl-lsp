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

//! `testutftonormalizeddstring` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testutftonormalizeddstring",
        surface: Some(SpecSurface::TCL91),
        arity: Arity::new(3, 4),
        hover: Some(HoverSnippet {
            summary: "Test UTF-8 Unicode normalisation into a DString (Tcl 9.1).",
            synopsis: &["testutftonormalizeddstring BYTES NORMALFORM PROFILE ?LENGTH?"],
            snippet: "Normalises ``BYTES`` to Unicode ``NORMALFORM`` under ``PROFILE`` into a ``Tcl_DString`` (optionally limited to ``LENGTH`` bytes).  Added in Tcl 9.1.",
            source: "Tcl test binary (tclTest.c)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
