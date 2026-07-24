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

//! `COMPRESS::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables compression for the current HTTP response.",
            synopsis: &["COMPRESS::enable (request | response)?"],
            snippet: "COMPRESS::enable\n    Enables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective.\n\nCOMPRESS::enable request\n    Enables compression for the current HTTP request. As with the normal COMPRESS::enable, you must enable the HTTP compression profile setting Selective Compression (version 11.x). You must also enable an HTTP profile with \"request-chunking selective\" selected.",
            source: "https://clouddocs.f5.com/api/irules/COMPRESS__enable.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::header Content-Type] contains \"text/html;charset=UTF-8\"} {\n    COMPRESS::enable\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "COMPRESS::enable (request | response)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
