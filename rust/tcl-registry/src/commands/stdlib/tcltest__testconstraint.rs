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

//! `tcltest::testConstraint` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::testConstraint",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Get or set a named test constraint boolean.",
            synopsis: &["tcltest::testConstraint constraint ?value?"],
            snippet: "With one argument, returns whether ``constraint`` is currently satisfied; with a boolean ``value``, sets it.  A ``test`` whose ``-constraints`` are not all satisfied is skipped.",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        // 0 = the constraint name being queried / set.
        arg_roles: &[(0, ArgRole::Name)],
        // The setter form `testConstraint NAME value` names a constraint the
        // outline lists; the value (arg 1) is its boolean condition, shown as
        // detail.  The one-argument getter form only reads the constraint, so
        // `defined_when_present(1)` records a symbol only for the setter.
        defines_symbol: Some(
            SymbolDef::new(0, DefinedSymbolKind::Constraint)
                .with_detail(1)
                .defined_when_present(1),
        ),
        ..CommandSpec::DEFAULT
    }
}
