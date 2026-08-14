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

//! `http::cookiejar` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::cookiejar",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create or configure an HTTP cookie jar (TclOO class).",
            synopsis: &[
                "http::cookiejar create name ?filename?",
                "http::cookiejar new ?filename?",
            ],
            snippet: "A TclOO class implementing RFC 6265 cookie management.  Instances track cookies received in HTTP responses and automatically attach them to subsequent requests.",
            source: "Tcl stdlib cookiejar package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("cookiejar"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
