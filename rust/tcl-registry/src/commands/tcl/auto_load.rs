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

//! `auto_load` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "auto_load",
        dialects: None,
        traits: Traits::OVERRIDABLE_LIBRARY_PROC,
        // `auto_load cmd ?namespace?` — the optional namespace argument makes
        // the real arity 1–2, not exactly 1 (verified against tclsh 9.0.4:
        // `auto_load foo ::ns` → 0, `auto_load a b c` → `wrong # args: should
        // be "auto_load cmd ?namespace?"`).
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Auto-load a command from the library",
            synopsis: &[],
            snippet: "",
            source: "Tcl man page library.n",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
