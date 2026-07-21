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

//! `ASM::deception` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::deception",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Mark a request as deceptive for further enforcement by asm",
            synopsis: &["ASM::deception"],
            snippet: "Mark a request as deceptive for further enforcement by asm",
            source: "https://clouddocs.f5.com/api/irules/ASM__deception.html",
            examples: "when ASM_REQUEST_DONE\n            {\n                ASM::deception\n            }",
            return_value: "no return value",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::deception",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
