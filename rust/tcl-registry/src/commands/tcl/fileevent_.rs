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
use tcl_dialect::model::{SpecSurface};

/// Command spec for `fileevent`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileevent",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel), (2, ArgRole::Body)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
target: SideEffectTarget::FileIo,
writes: true,
..SideEffect::DEFAULT
}],
        hover: Some(HoverSnippet::brief(
            "Execute a script when a channel becomes readable or writable.",
            &[
                "fileevent channel readable ?script?",
                "fileevent channel writable ?script?",
            ],
            "Tcl fileevent(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
