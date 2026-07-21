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

//! `TAP::insight` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::insight",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Accumulates or sends key:value pairs to TAP, returns token.",
            synopsis: &[
                "TAP::insight set (TAP_INSIGHT_KEY TAP_INSIGHT_VALUE)*",
                "TAP::insight send TAP_INSIGHT_EVENT_TYPE TAP_INSIGHT_REASON",
            ],
            snippet: "With arguments accumulates them as key:value pairs, without arguments sends accumulated to TAP.\nReturns token supplied by TAP service.",
            source: "https://clouddocs.f5.com/api/irules/TAP__insight.html",
            examples: "when TAP_REQUEST {\n    if { ([TAP::insight] eq \"block\") } {\n        drop\n    }\n}",
            return_value: "Returns one of the following actions: allow, block, captcha, conviction, deception, timeout.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TAP::insight set (TAP_INSIGHT_KEY TAP_INSIGHT_VALUE)*",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
