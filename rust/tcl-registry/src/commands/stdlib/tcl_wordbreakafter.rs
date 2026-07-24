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

//! `tcl_wordBreakAfter` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_wordBreakAfter",
        traits: Traits::PURE | Traits::OVERRIDABLE_LIBRARY_PROC,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Return the index of the first word boundary after *start* in *str*.",
            synopsis: &["tcl_wordBreakAfter str start"],
            snippet: "",
            source: "Tcl stdlib auto-loaded utility",
            examples: "",
            return_value: "",
        }),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
