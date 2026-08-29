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

//! `tcltest::loadScript` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::loadScript",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Get or set the load script.  Deprecated: use ``configure -load``.",
            synopsis: &["tcltest::loadScript ?script?"],
            snippet: "With no argument, returns the current load script; with ``script``, sets the Tcl script run to load the tested commands into the interpreter.",
            source: "Tcl stdlib tcltest package (deprecated)",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        // The optional value is a Tcl script the harness later evaluates.
        arg_roles: &[(0, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        // `DEFERS_BODY` — "later evaluates" said in data. tclsh 8.6.16 /
        // 9.0.4, byte-identical: `proc p {} { tcltest::loadScript {error
        // stop}; set ::reached 1 }` sets `::reached` (issue #1672 audit).
        traits: Traits::DEFERS_BODY,
        deprecated_replacement: Some("tcltest::configure"),
        ..CommandSpec::DEFAULT
    }
}
