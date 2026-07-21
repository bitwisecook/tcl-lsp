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

//! `fileutil::updateInPlace` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::updateInPlace ?options? fileName cmdOrBody",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::updateInPlace",
        dialects: None,
        arity: Arity::at_least(2),
        // The trailing `cmdOrBody` argument is invoked as a
        // command prefix with the file contents appended at runtime.
        // Static arity checks must relax the proc's required arity
        // by 1 when checking the callback (see `e30b6ae9`, `#308`).
        body_arg_implicit_args: 1,
        hover: Some(HoverSnippet {
            summary: "Update a file in place using a command.",
            synopsis: &["fileutil::updateInPlace ?options? fileName cmdOrBody"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
