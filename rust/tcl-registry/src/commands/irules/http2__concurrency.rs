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

//! `HTTP2::concurrency` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::concurrency",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to determine the number of active concurrent streams in the current HTTP/2 session.",
            synopsis: &["HTTP2::concurrency"],
            snippet: "Returns number of active concurrent streams in the current HTTP/2 session.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__concurrency.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"Number of active concurrent streams is [HTTP2::concurrency]\"\n}",
            return_value: "The return is a number indicating the active concurrent streams in current HTTP/2 session.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "HTTP2::concurrency",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Http2State,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
