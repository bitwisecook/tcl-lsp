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

//! `HTTP2::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the HTTP2 filter from passthrough to full parsing mode.",
            synopsis: &["HTTP2::enable ('clientside')? ('serverside')?"],
            snippet: "Changes the HTTP2 filter from passthrough to full parsing mode. This\ncommand is useful when it is unknown whether HTTP2 will be used at\nconfiguration time, but known at runtime.  This command is quite tricky\nto get right.  In general, it may be better to use HTTP2::disable.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__enable.html",
            examples: "when CLIENT_ACCEPTED {\n    HTTP2::disable\n    if { !([IP::addr [IP::client_addr] eq 10.0.0.0/8]) } {\n        HTTP2::enable\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP2::enable ('clientside')? ('serverside')?",
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
