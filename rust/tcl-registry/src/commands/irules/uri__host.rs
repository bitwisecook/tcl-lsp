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

//! `URI::host` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::host",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the host portion of a given URI.",
            synopsis: &["URI::host URI_STRING"],
            snippet: "Returns the host portion of a given URI.",
            source: "https://clouddocs.f5.com/api/irules/URI__host.html",
            examples: "when RULE_INIT {\n        # Loop through some test URLs and URIs and log the URI::host value\n        foreach uri [list \\\n                http://example.com/file.ext \\\n                http://example.com:80/file.ext \\\n                https://example.com:443/file.ext \\\n                ftp://example.com/file.ext \\\n                sip://example.com/file.ext \\\n                myproto://example.com/file.ext \\\n                /example.com \\\n                /uri?url=http://example.com/uri \\\n        ] {",
            return_value: "Returns the host portion of a given URI.",
        }),
        forms: &[FormSpec {
            synopsis: "URI::host URI_STRING",
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
