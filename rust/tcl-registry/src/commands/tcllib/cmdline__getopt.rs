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

//! `cmdline::getopt` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::getopt argvVar optstring optVar valVar",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getopt",
        dialects: None,
        arity: Arity::exact(4),
        hover: Some(HoverSnippet {
            summary: "Parse a single command-line option.",
            synopsis: &["cmdline::getopt argvVar optstring optVar valVar"],
            snippet: "Processes a single option from the argument list. Returns 1 if an option was found, 0 if no more options, or -1 on error.",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "1 on success, 0 when done, -1 on error.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
