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

//! `bind` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Event text which Tk substitutes from the user event. `%A` carries the
/// character payload. `%K` (keysym), `%W`, coordinates, and modifiers are
/// framework metadata rather than arbitrary user text and remain clean.
const USER_EVENT_INPUTS: &[CallbackTaintInput] = &[CallbackTaintInput::TK_EVENT_CHAR];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "bind tag ?sequence? ?+??command?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bind",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::new(1, 3),
        // The `bind tag sequence script` form's trailing script is a
        // deferred event-handler body (runs from the Tk event loop, not
        // the caller's frame), so recurse into it for highlighting and
        // treat it as structural.
        // Optional index 2 is present only in the full binding form.
        // Neither its role nor deferred timing depends on earlier value bytes.
        arg_roles: &[(2, ArgRole::Body)],
        callback_taint_inputs: &[(2, USER_EVENT_INPUTS)],
        body_kind: BodyKind::Structural,
        // `DEFERS_BODY` — the "deferred event-handler body" above, said where
        // a consumer asking "can this body stop control reaching my next
        // statement?" can read it. Documentary rather
        // than oracle-measured: this environment is headless, so `package
        // require Tk` is unavailable. Tk's `bind.n` is unambiguous — the
        // script is *associated* with the tag and sequence, and "will be
        // evaluated whenever the given event sequence occurs".
        traits: Traits::DEFERS_BODY,
        hover: Some(HoverSnippet {
            summary: "Arrange for X event bindings on windows or tags.",
            synopsis: &[
                "bind tag",
                "bind tag sequence",
                "bind tag sequence script",
                "bind tag sequence +script",
            ],
            snippet: "",
            source: "Tk man page bind.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
