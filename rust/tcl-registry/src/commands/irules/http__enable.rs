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

//! `HTTP::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the HTTP filter from passthrough to full parsing mode.",
            synopsis: &["HTTP::enable"],
            snippet: "Changes the HTTP filter from passthrough to full parsing mode. This\ncould be useful, for instance, if you need to determine whether or not\nHTTP is passing over the connection and enable the HTTP filter\nappropriately, or if you have a protocol that is almost but not quite\nlike HTTP, and you need to re-enable HTTP parsing after temporarily\ndisabling it.\nUse of this command can be extremely tricky to get exactly right; its\nuse is not recommended in the majority of cases.\nNote: This command does not function in certain versions of BIG-IP\n(v9.4.0 - v9.4.4).",
            source: "https://clouddocs.f5.com/api/irules/HTTP__enable.html",
            examples: "when HTTP_REQUEST {\nlog local0. \"Got request: [HTTP::uri]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "HTTP::enable",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
