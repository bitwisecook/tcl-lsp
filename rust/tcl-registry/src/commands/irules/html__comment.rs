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

//! `HTML::comment` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::comment",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Query and update HTML comment.",
            synopsis: &["HTML::comment ((append STRING) | (prepend STRING) | remove)?"],
            snippet: "Queries, removes HTML comment or appends/prepends it by a string.\n\nHTML::comment\nReturn the entire HTML comment, including the opening and the closing delimiter.\n\nHTML::comment append <string>\nInsert a string after the closing delimiter of the HTML comment; when multiple appends are issued, the inserted strings are ordered according to the sequence of the append commands as they are issued for the given comment.",
            source: "https://clouddocs.f5.com/api/irules/HTML__comment.html",
            examples: "when HTML_COMMENT_MATCHED {\n    HTML::comment append \"some_string\"\n}",
            return_value: "HTML::comment returns the entire HTML comment; others do not return anything.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTML"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "HTML::comment ((append STRING) | (prepend STRING) | remove)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
