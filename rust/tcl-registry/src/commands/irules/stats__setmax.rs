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

//! `STATS::setmax` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::setmax",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Ensures that the value of a Statistics profile setting is at the least value.",
            synopsis: &["STATS::setmax PROFILE_NAME FIELD_NAME (VALUE)?"],
            snippet: "Ensures that the value of the specified Statistics profile setting\n(field) is at the least value.",
            source: "https://clouddocs.f5.com/api/irules/STATS__setmax.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "STATS::setmax PROFILE_NAME FIELD_NAME (VALUE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IStats,
            writes: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
