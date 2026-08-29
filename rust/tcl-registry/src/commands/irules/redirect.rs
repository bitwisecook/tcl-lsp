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

//! `redirect` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "redirect",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Redirects an HTTP request to the specific location.",
            synopsis: &["redirect to HOST_URI"],
            snippet: "Redirects an HTTP request to a specific location. The location can be\neither a host name or a URI. This is a BIG-IP 4.X statement, provided\nfor backward compatibility. You can use the equivalent 9.X command\nHTTP::redirect instead.",
            source: "https://clouddocs.f5.com/api/irules/redirect.html",
            examples: "when HTTP_REQUEST {\n    # HTTP::redirect, HTTP::host and HTTP::uri should be used instead\n    redirect to \"https://[http_host][http_uri]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "redirect to HOST_URI",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ResponseCommit,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("HTTP::redirect"),
        ..CommandSpec::DEFAULT
    }
}
