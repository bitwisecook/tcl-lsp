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

//! `HTTP::is_redirect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::is_redirect",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Returns a true value if the response is a redirect.",
            synopsis: &["HTTP::is_redirect"],
            snippet: "Returns a true value if the response is a redirect. Since only\nresponses can be redirects, it does not make sense to use this command\nin a clientside event.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__is_redirect.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::is_redirect] } {\n    log local0. \"Request redirected.\"\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "HTTP::is_redirect",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpStatus,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
