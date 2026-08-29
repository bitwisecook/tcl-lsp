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

//! `URI::basename` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::basename",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Extracts the basename part of a given uri string.",
            synopsis: &["URI::basename URI_STRING"],
            snippet: "Extracts the basename part of a given uri string.\nFor the following URI:\n/main/index.jsp?user=test&login=check\n\nThe basename is:\n\nindex.jsp",
            source: "https://clouddocs.f5.com/api/irules/URI__basename.html",
            examples: "when HTTP_REQUEST {\n  set base [URI::basename [HTTP::uri]]\n  log local0. \"Basename of uri [HTTP::uri] is $base\"\n}",
            return_value: "Return the basename part of a given uri string.",
        }),
        forms: &[FormSpec {
            synopsis: "URI::basename URI_STRING",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
