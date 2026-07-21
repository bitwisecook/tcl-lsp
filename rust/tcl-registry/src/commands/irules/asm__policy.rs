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

//! `ASM::policy` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the name of the ASM security policy that was applied for the request.",
            synopsis: &["ASM::policy"],
            snippet: "Returns the name of the ASM policy that was applied on the request. It can be used to detect which CPM rules are applied or ASM::enable commands are applied on a request.",
            source: "https://clouddocs.f5.com/api/irules/ASM__policy.html",
            examples: "when ASM_REQUEST_BLOCKING{\n    log local0. \"The request was blocked using the [ASM::policy] policy\"\n}",
            return_value: "Returns the ASM policy applied on the request or null string if ASM is disabled.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::policy",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
