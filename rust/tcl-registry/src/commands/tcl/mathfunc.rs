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

//! `tcl::mathfunc` — namespace of `expr` math-function commands.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::mathfunc",
        traits: Traits::PURE,
        // TIP 232 added the namespace (and this command-table mechanism)
        // in Tcl 8.5 — see mathfunc_generated.rs's module docs for why
        // that's the floor for every function regardless of when its own
        // `expr` grammar entry appeared.
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(0),
        return_type: Some(TclType::Numeric),
        hover: Some(HoverSnippet::brief(
            "Mathematical function commands.",
            &["tcl::mathfunc::name ?arg ...?"],
            "Tcl mathfunc(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
