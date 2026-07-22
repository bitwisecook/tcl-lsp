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

//! `fileevent` — execute a script when a channel becomes readable or writable.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileevent channel readable ?script?",
}];

/// Command spec for `fileevent`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileevent",
        dialects: None,
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel), (2, ArgRole::Body)],
        // The script fires later, when the channel becomes ready — a
        // different call frame than the one that registered it (same
        // reasoning as `after`'s `body_kind`, which see).
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Execute a script when a channel becomes readable or writable",
            synopsis: &[
                "fileevent channel readable ?script?",
                "fileevent channel writable ?script?",
            ],
            snippet: "The fileevent command has been superseded by the chan event command which supports the same syntax and options.",
            source: "Tcl man page fileevent.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
