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

//! `MR::max_retries` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::max_retries",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the number of retries allows for this router instance.",
            synopsis: &["MR::max_retries"],
            snippet: "returns the number of retries allowed",
            source: "https://clouddocs.f5.com/api/irules/MR__max_retries.html",
            examples: "when MR_FAILED {\n    if {[MR::message retry_count] < [MR::max_retries]} {\n        MR::message nexthop none\n        MR::retry\n    }\n}",
            return_value: "returns the number of retries allowed",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MR::max_retries",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
