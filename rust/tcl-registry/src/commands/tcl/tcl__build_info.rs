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

//! `tcl::build-info` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::build-info",
        dialects: None,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Return compile-time build metadata for the Tcl runtime.",
            synopsis: &["tcl::build-info ?key?"],
            snippet: "Returns the compile-time build metadata for the running Tcl runtime.  With no arguments, returns the patchlevel.  With a key argument, returns the value associated with that key (e.g. ``version``, ``commit``, ``branch``, ``compiler``).",
            source: "Tcl tcl::build-info (internal)",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "tcl::build-info ?key?",
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
