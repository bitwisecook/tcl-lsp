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

//! `tcltest::bytestring` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::bytestring",
        // Not defined under Tcl 9.0+ (guarded out of tcltest 2.5.10).
        surface: Some(SpecSurface::TCL8X),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Convert a string to its byte representation (Tcl < 9.0).",
            synopsis: &["tcltest::bytestring string"],
            snippet: "Equivalent to ``encoding convertfrom identity``.  Not exported in Tcl 9.0+.",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        deprecated_replacement: Some("encoding convertfrom identity"),
        ..CommandSpec::DEFAULT
    }
}
