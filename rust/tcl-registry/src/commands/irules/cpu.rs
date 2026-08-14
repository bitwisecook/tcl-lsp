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

//! `cpu` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "cpu",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the average TMM cpu load for the given interval.",
            synopsis: &["cpu usage ("],
            snippet: "The cpu usage command returns the average TMM cpu load for the given\ninterval. All averages are exponential weighted moving averages over\nthe interval.",
            source: "https://clouddocs.f5.com/api/irules/cpu.html",
            examples: "when HTTP_REQUEST {\n  if{ [cpu usage 5sec] <= 1} {\n    pool1\n  } else {\n    HTTP::redirect \"http://anotherpool.com\"\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "cpu usage (",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
