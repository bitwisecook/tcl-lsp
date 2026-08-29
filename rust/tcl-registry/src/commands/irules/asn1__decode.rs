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

//! `ASN1::decode` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::decode",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Decodes ASN.1 records.",
            synopsis: &["ASN1::decode ELEMENT FORMAT (VAR_NAME)*"],
            snippet: "This command is used to decode ASN.1 records. element specifies the\ndata to decode. It can be either a byte array (like is returned from\nTCP::payload), or an element object returned by one of the other\ncommands. The bytes are decoded according to formatString, and the\nresults are stored in variables whose names are provided as command\narguments. If a variable with a name specified doesn't exist, it will\nbe created in the current scope. If the variable does exist, it's value\nwill be overwritten. The command returns the number of elements\ndecoded.",
            source: "https://clouddocs.f5.com/api/irules/ASN1__decode.html",
            examples: "ASN1::decode $ele \"?a?aa?b\" ruleId type matchValue dnAttrs\nif {![info exists ruleId] && ![info exists type]} {\n  log local0. \"ERR: extensibleMatch must contain either a matchingRule or type component\"\n}\n# Handle default value for dnAttributes component\nif {![info exists dnAttrs]} {\n  set dnAttrs 0\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ASN1::decode ELEMENT FORMAT (VAR_NAME)*",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
