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

//! `CACHE::headers` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::headers",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the HTTP headers of the object in the cache.",
            synopsis: &["CACHE::headers"],
            snippet: "Returns the HTTP headers of the object in the cache.\nIf CACHE::header is used to manipulate the response headers prior to calling CACHE::headers, the modifications will not be reflected by CACHE::headers.\n\nCACHE::headers\n\n     * Returns the HTTP headers of the object in the cache as TCL Name / value pairs list.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__headers.html",
            examples: "when CACHE_RESPONSE {\n  # log all  HTTP headers sent in cache response.\n  log local0. [CACHE::headers]\n}",
            return_value: "Returns the HTTP headers of the object in the cache as TCL Name / value pairs list.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CACHE"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "CACHE::headers",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
