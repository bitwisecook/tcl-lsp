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

//! `matchclass` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "matchclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Performs comparison against the contents of data group.",
            synopsis: &["matchclass CLASS_OR_VALUE KEYWORDS VALUE_OR_CLASS"],
            snippet: "Performs comparisons against the contents of data group. Typically used\nfor conditional logic control.\n\nNote: matchclass has been deprecated in v10 in favor of the new\nclass commands. The class command offers better functionality and\nperformance than matchclass.\n\nNote that you should not use a $:: or :: prefix on the datagroup name\nwhen using the matchclass command (or in any datagroup reference on\n9.4.4 or later).\n\nIn v9.4.4 - 10, using $::datagroup_name will work but demote the\nvirtual server from running on all TMMs. For details, see the CMP\ncompatibility page.",
            source: "https://clouddocs.f5.com/api/irules/matchclass.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [matchclass [IP::remote_addr] equals aol] } {\n     pool aol_pool\n  } else {\n     pool all_pool\n }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "matchclass CLASS_OR_VALUE KEYWORDS VALUE_OR_CLASS",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DataGroup,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("class"),
        ..CommandSpec::DEFAULT
    }
}
