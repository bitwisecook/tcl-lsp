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

//! `gets` — read a line from a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "gets channel ?varName?",
    dialects: None,
}];

/// Command spec for `gets`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "gets",
        dialects: None,
        traits: Traits::BYTE_COMPILED | Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::VarWrite)],
        assigns_variable_at: Some(1),
        return_type: Some(TclType::String),
        // The two-arg `gets chan varName` form writes the read *line* (a
        // String) to its target while returning the character *count* (an
        // Int).  Type the target as the line it always receives, not the
        // count (issue #867).
        var_write_typing: VarWriteTyping::Fixed(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Read a line from a channel",
            synopsis: &["gets channel ?varName?"],
            snippet: "The gets command has been superceded by the chan gets command which supports the same syntax and options.",
            source: "Tcl man page gets.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
