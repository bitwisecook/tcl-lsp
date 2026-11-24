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

//! `http::formatQuery` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::formatQuery",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::any(),
        arity_windows: const {
            &[crate::arity::ArityWindow {
                lifecycle: crate::lifecycle::Lifecycle::introduced_in("2.9"),
                arity: Arity::stepped(0, Arity::UNLIMITED, 2),
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Generate an x-url-encoded query string from key/value pairs.",
            synopsis: &["http::formatQuery ?key value ...?"],
            snippet: "Takes alternating key/value arguments and returns a properly URL-encoded query string suitable for use as a ``-query`` value in ``http::geturl``.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..CommandSpec::DEFAULT
    }
}
