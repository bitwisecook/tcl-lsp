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

//! `URI::protocol` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the protocol of the given URI.",
            synopsis: &["URI::protocol URI_STRING"],
            snippet: "Returns the protocol of the given URI.",
            source: "https://clouddocs.f5.com/api/irules/URI__protocol.html",
            examples: "when RULE_INIT {\n        # Loop through some test URLs and URIs and log the URI::protocol value\n        foreach uri [list \\\n                http://test.com \\\n                https://test.com \\\n                ftp://test.com \\\n                sip://test.com \\\n                myproto://test.com \\\n                /test.com \\\n                /uri?url=http://test.example.com/uri \\\n        ] {\n                log local0. \"\\[URI::protocol $uri\\]: [URI::protocol $uri]\"\n        }\n}",
            return_value: "Returns the protocol of the given URI.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "URI::protocol URI_STRING",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
