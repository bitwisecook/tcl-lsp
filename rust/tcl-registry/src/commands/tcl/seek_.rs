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

//! `seek` — set the access position for a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "seek channelId offset ?origin?",
}];

/// Command spec for `seek`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "seek",
        dialects: None,
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Set the access position for a channel.",
            synopsis: &["seek channelId offset ?origin?"],
            snippet: "Default origin is `start`. Returns empty string.",
            source: "Tcl man page seek.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
