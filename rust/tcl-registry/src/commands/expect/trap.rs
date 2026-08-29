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

//! `trap` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "trap ?command? ?signal ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trap",
        surface: Some(SpecSurface::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Trap signals and execute a command when they occur.",
            synopsis: &[
                "trap ?command? ?signal ...?",
                "trap SIG_IGN SIGINT",
                "trap { puts caught } SIGTERM",
            ],
            snippet: "",
            source: "Expect trap(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    }
}
