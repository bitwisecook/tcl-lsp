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

//! `findstr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "findstr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Finds a string within another string and returns the string starting at the offset specified from the match.",
            synopsis: &["findstr STRING SEARCH_STRING ("],
            snippet: "A custom iRule function which finds a string within another string\nand returns the string starting at the offset specified from the match.",
            source: "https://clouddocs.f5.com/api/irules/findstr.html",
            examples: "when RULE_INIT {\n  set static::payload {<meta HTTP-EQUIV=\"REFRESH\" CONTENT=\"0; URL=https://host.domain.com/path/file.ext?...&var=val\">}\n  set static::term {\">}\n  set urlresponse [findstr $static::payload URL= 4 $static::term]\n  log local0. \"urlresponse $urlresponse\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "findstr STRING SEARCH_STRING (",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
