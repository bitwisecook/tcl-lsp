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

//! `cmdline::typedGetopt` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "cmdline::typedGetopt argvVar optstring optVar valVar",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::typedGetopt",
        dialects: None,
        arity: Arity::exact(4),
        hover: Some(HoverSnippet {
            summary: "Parse a single typed command-line option.",
            synopsis: &["cmdline::typedGetopt argvVar optstring optVar valVar"],
            snippet: "",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "1 on success, 0 when done, -1 on error.",
        }),
        forms: FORMS,
        arg_roles: &[
            (0, ArgRole::VarWrite),
            (2, ArgRole::VarWrite),
            (3, ArgRole::VarWrite),
        ],
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
