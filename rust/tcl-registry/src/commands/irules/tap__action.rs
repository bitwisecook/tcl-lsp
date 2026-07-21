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

//! `TAP::action` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or updates security token action.",
            synopsis: &[
                "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
            ],
            snippet: "Returns action supplied by TAP service. If supplied new action to set function returns previous action.",
            source: "https://clouddocs.f5.com/api/irules/TAP__action.html",
            examples: "when TAP_REQUEST {\n    if {    ([TAP::action] eq \"block\") } {\n        drop\n    }\n}",
            return_value: "Returns one of the following actions: allow, alarm, basicPolicy, strictPolicy, jsInection, captcha, block, tcpReset, deception.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["TAP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?",
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
