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

//! `CACHE::header` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::header",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get/modify the content of an header related to an object stored in the RAM Cache.",
            synopsis: &[
                "CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME",
                "CACHE::header ('insert' | 'replace') HEADER_NAME HEADER_VALUE",
            ],
            snippet: "The command is used to gather or modify the content of a header stored\nin the cache.\n\nCACHE::header <name>\n\n     * Get the content of the requested header\n\nCACHE::header insert <name> <value>\n\n     * Add the header with the specified value to the list of headers sent to the\n       client when delivering an object from the cache.\n\nCACHE::header remove <name>\n\n     * Remove the header with the specified name.\n\nCACHE::header replace <name> <value>\n\n     * Replace the header with the specified value.\n\nCACHE::header value <name>\n\n     * Return the header value for the specified header name.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__header.html",
            examples: "when CACHE_UPDATE {\n    # cached object's headers manipulation\n    # modifications will be seen whenever the object is served from cache\n    CACHE::header replace Server Big-IP-Server\n}",
            return_value: "",
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
            synopsis: "CACHE::header ('exists' | 'remove' | 'value') HEADER_NAME",
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
