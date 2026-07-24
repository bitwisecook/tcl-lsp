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

//! `pkg_mkIndex` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg_mkIndex",
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Build a ``pkgIndex.tcl`` file for one or more packages.",
            synopsis: &[
                "pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern ...?",
            ],
            snippet: "Scans *dir* for Tcl source and binary files matching *pattern* (default ``*.tcl *.{so,dll}``) and builds a ``pkgIndex.tcl`` that enables ``package require`` to find them.",
            source: "Tcl stdlib package utilities",
            examples: "",
            return_value: "",
        }),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
