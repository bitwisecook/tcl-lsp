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

//! `http_cookie` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_cookie",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of the specified HTTP cookie.",
            synopsis: &["http_cookie ANY_CHARS"],
            snippet: "Returns the value in the Cookie: header for the specified cookie\nname. This is a BIG-IP version 4.X variable, provided for\nbackward-compatibility. You can use the equivalent 9.X command\nHTTP::cookie instead",
            source: "https://clouddocs.f5.com/api/irules/http_cookie.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "http_cookie ANY_CHARS",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpCookie,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("HTTP::cookie"),
        deprecated_replacement_drop_in: true,
        ..CommandSpec::DEFAULT
    }
}
