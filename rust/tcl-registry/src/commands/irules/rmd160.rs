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

//! `rmd160` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "rmd160",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the RIPEMD-160 message digest of the specified string.",
            synopsis: &["rmd160 ANY_CHARS"],
            snippet: "Returns the RIPEMD-160 (RACE Integrity Primitives Evaluation Message Digest) message digest of the specified string, or an empty string if an error occurs. Used to ensure data integrity.",
            source: "https://clouddocs.f5.com/api/irules/rmd160.html",
            examples: "when HTTP_REQUEST {\n    binary scan [rmd160 [HTTP::host]] w1 key\n\n    set key [expr {$key & 1}]\n    switch $key {\n        0 { pool my_pool member 1.2.3.4:80 }\n        1 { pool my_pool member 5.6.7.8:80 }\n    }\n}",
            return_value: "rmd160 <string> Returns the RIPEMD-160 message digest of the specified string, or an empty string if an error occurs.",
        }),
        forms: &[FormSpec {
            synopsis: "rmd160 ANY_CHARS",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
