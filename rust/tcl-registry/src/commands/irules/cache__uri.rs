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

//! `CACHE::uri` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Overrides the URI value used by the cache to store the cached content.",
            synopsis: &["CACHE::uri URI_STRING"],
            snippet: "By default, cached content is stored with a unique key that consists of the Host\nheader, URI, Accept-Encoding and User-Agent. If multiple variations of the\nsame content must be cached under specific conditions (different client), you\ncan use this command to modify URI and create a unique key, thus creating\ncached content specific to that condition. This can be used to prevent one user\nor group's cached data from being served to different users/groups.\n\nIf any of the key elements is rewritten at the HTTP level (HTTP_REQUEST),\nthe key uses those values, not the original values.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__uri.html",
            examples: "when HTTP_REQUEST {\n  set my_session_string <unique, sanitized session value>\n  CACHE::uri [HTTP::uri]$my_session_string\n}",
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
            synopsis: "CACHE::uri URI_STRING",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
