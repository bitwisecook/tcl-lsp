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

//! `ROUTE::domain` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::domain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the current routing domain of the current connection.",
            synopsis: &["ROUTE::domain"],
            snippet: "Returns the current routing domain of the current connection. Several\ncommands allow an addition rt_domain option: node, snat, LB::status",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__domain.html",
            examples: "when CLIENT_ACCEPTED {\n    set gateway 10.3.1.11\n    set bandwidth [ROUTE::bandwidth [IP::remote_addr] $gateway%[ROUTE::domain]]\n    if { $bandwidth > 0 } {\n        log local0. \"Destination found in cache, bandwidth = $bandwidth\"\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ROUTE::domain",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
