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

//! `ISTATS::incr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::incr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Increments the specified key by the given value.",
            synopsis: &["ISTATS::incr KEY VALUE"],
            snippet: "Increments the specified key by the given value. The increment value must be non-negative for a counter.\n\nNote that text string iStats may not be incremented.",
            source: "https://clouddocs.f5.com/api/irules/ISTATS__incr.html",
            examples: "when HTTP_REQUEST {\n        if { [string tolower [HTTP::uri]] equals \"/12345\" } {\n                ISTATS::incr \"uri /12345 counter Requests\" 1\n                HTTP::uri \"/\"\n                HTTP::redirect \"http://www.mysite.com\"\n        } elseif { [string tolower [HTTP::uri]] equals \"/stats\" } {\n                  HTTP::respond 200 content \"<html><body>Requests for /12345: [ISTATS::get \"uri /12345 counter Requests\"]</body></html>\"\n        }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ISTATS::incr KEY VALUE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IStats,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
