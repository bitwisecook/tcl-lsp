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

//! `uuid::uuid` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "equal",
        arity: Arity::exact(2),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "generate",
        arity: Arity::exact(0),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uuid::uuid subcommand ?args?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uuid::uuid",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Generate or manipulate UUIDs.",
            synopsis: &["uuid::uuid generate", "uuid::uuid equal id1 id2"],
            snippet: "Generate RFC 4122 version 4 universally unique identifiers.",
            source: "tcllib uuid package",
            examples: "set id [uuid::uuid generate]",
            return_value: "A UUID string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        tcllib_package: Some("uuid"),
        ..CommandSpec::DEFAULT
    }
}
