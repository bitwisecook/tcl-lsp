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

//! `SSL::allow_dynamic_record_sizing` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::allow_dynamic_record_sizing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set dynamic record sizing.",
            synopsis: &["SSL::allow_dynamic_record_sizing (ZERO_ONE)?"],
            snippet: "SSL::allow_dynamic_record_sizing\n  Returns the currently set value for allowing dynamic record sizing\nSSL::allow_dynamic_record_sizing ( 0 | 1 )\n  0 disables dynamic record sizing, 1 enables it.\n  Dynamic record sizing, when using protocols such as HTTP, can increase respnonsiveness of a website.",
            source: "https://clouddocs.f5.com/api/irules/SSL__allow_dynamic_record_sizing.html",
            examples: "when CLIENT_ACCEPTED {\n    SSL::allow_dynamic_record_sizing 1\n}",
            return_value: "SSL::allow_dynamic_record_sizing Returns the currently set dynamic record sizing value. SSL::allow_dynamic_record_sizing [0|1] There is no return value.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::allow_dynamic_record_sizing (ZERO_ONE)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
