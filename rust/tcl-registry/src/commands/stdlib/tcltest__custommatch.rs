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

//! `tcltest::customMatch` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

fn custom_match_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (args.len() >= 2)
        .then_some((1, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::customMatch",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Register a custom matching command for test results.",
            synopsis: &["tcltest::customMatch mode command"],
            snippet: "Registers ``mode`` as a value for ``test -match``.  ``command`` is a command prefix invoked as ``command expected actual`` and must return a boolean.",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        // 0 = the new mode name; 1 = a command prefix invoked as
        // `command expected actual` → 2 appended args (`Exactly(2)`).
        arg_roles: &[(0, ArgRole::Name)],
        command_prefixes: &[(1, AppendedArity::Exactly(2))],
        script_timing_resolver: Some(custom_match_script_timing),
        // `customMatch MODE command` always defines a new match mode; the
        // backing command (arg 1) is shown as the outline detail.
        defines_symbol: Some(SymbolDef::new(0, DefinedSymbolKind::Matcher).with_detail(1)),
        ..CommandSpec::DEFAULT
    }
}
