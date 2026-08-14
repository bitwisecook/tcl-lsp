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

//! `DOSL7::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables blocking and detection of DoS attacks according to the ASM security policy configuration.",
            synopsis: &["DOSL7::disable"],
            snippet: "Disables blocking and detection of DoS attacks according to the ASM\nsecurity policy configuration. When enabled using DOSL7::enable,\ntransactions will be enforced according to the DoS L7 ASM policy\nconfiguration for both detection and prevention.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__disable.html",
            examples: "when IN_DOSL7_ATTACK {\n    DOSL7::disable\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "DOSL7::disable",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Dosl7State,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
