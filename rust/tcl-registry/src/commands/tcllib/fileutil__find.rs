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

//! `fileutil::find` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "fileutil::find ?basedir? ?filtercmd?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::find",
        dialects: None,
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Recursively find files matching a filter.",
            synopsis: &["fileutil::find ?basedir? ?filtercmd?"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        // `filtercmd` (index 1) is invoked as `filtercmd name` for each
        // candidate file/dir → 1 appended arg.  The registry drops the entry
        // for the shorter `?basedir?`-only and zero-arg query forms (idx≥argc),
        // so a static table is safe here.
        command_prefixes: &[(1, AppendedArity::Exactly(1))],
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
