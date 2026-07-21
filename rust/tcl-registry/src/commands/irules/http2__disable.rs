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

//! `HTTP2::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the HTTP2 filter from full parsing to passthrough mode.",
            synopsis: &["HTTP2::disable ('clientside')? ('serverside')? ('discard')?"],
            snippet: "Changes the HTTP2 filter from full parsing to passthrough mode. This\ncommand is useful when using an HTTP2 profile with an application that\nproxies data over HTTP.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__disable.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] contains \"http1_backend\"} {\n        HTTP2::disable serverside\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP2::disable ('clientside')? ('serverside')? ('discard')?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Http2State,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
