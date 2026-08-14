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

//! `STATS::incr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::incr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Increments the value of a Statistics profile setting.",
            synopsis: &["STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?"],
            snippet: "Increments the value of the specified setting (field), in the specified\nStatistics profile, by the specified value. If you do not specify a\nvalue, the system increments by 1. It is possible to set a negative\nvalue in order to decrement the counter. Returns the current value of\nthe field which was incremented.",
            source: "https://clouddocs.f5.com/api/irules/STATS__incr.html",
            examples: "when HTTP_REQUEST {\n\n   # Increment the number of unanswered HTTP requests\n   log local0. \"Incremented the current count to: [STATS::incr my_stats_profile_name \"current_count\"]\"\n}",
            return_value: "Returns the current value of the field which was incremented.",
        }),
        forms: &[FormSpec {
            synopsis: "STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?",
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
