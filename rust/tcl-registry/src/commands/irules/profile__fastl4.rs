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

//! `PROFILE::fastL4` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::fastL4",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of a Fast L4 profile setting.",
            synopsis: &["PROFILE::fastL4 ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned Fast L4 profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__fastL4.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned Fast L4 profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::fastL4 ATTR",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
