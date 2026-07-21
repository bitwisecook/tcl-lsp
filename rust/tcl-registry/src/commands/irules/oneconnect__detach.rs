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

//! `ONECONNECT::detach` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::detach",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Detaches server-side OneConnect connections.",
            synopsis: &["ONECONNECT::detach BOOL_VALUE"],
            snippet: "Controls the behavior of a server-side connection when a OneConnect\nprofile is on the virtual server. The default behavior is that the\nserver-side connection detaches after each response is completed, and a\nnew load balancing decision and persistence look-up are performed for\nevery request.\nDisabling detaching prevents this behavior.\nNote: the use of the terms \"request\" and \"response\" imply the presence\nof a supported layer 7 profile (e.g. the HTTP profile) on the virtual\nserver. An iRule can also detaching the server-side connection using\nthe LB::detach command.",
            source: "https://clouddocs.f5.com/api/irules/ONECONNECT__detach.html",
            examples: "when HTTP_RESPONSE {\n    if { $headreq } {\n        # Response to HEAD request. Detach after done.\n        ONECONNECT::detach enable\n        ONECONNECT::reuse enable\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ONECONNECT::detach BOOL_VALUE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
