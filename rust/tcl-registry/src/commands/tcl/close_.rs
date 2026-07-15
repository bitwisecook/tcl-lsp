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

//! `close` — close an open channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "close channelId ?r(ead)|w(rite)?",
}];

/// Command spec for `close`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        // `FIRE_AND_FORGET_TEARDOWN`: `Tcl_CloseObjCmd` (tclIOCmd.c) unregisters
        // and frees the channel — a second `close` on the same handle errors
        // ("can not find channel named …"), which is why a bare
        // `catch {close $h}` is the documented fire-and-forget idiom the
        // W302 suppression keys off this trait.
        traits: Traits::BYTE_COMPILED | Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Close a channel.",
            synopsis: &["close channelId ?r(ead)|w(rite)?"],
            snippet: "For bidirectional pipelines you may close one direction (`read`/`write`) selectively.",
            source: "Tcl close(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
