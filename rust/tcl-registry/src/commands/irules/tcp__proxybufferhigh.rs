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

//! `TCP::proxybufferhigh` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::proxybufferhigh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets proxy buffer high threshold.",
            synopsis: &["TCP::proxybufferhigh"],
            snippet: "Gets the threshold at which the proxy buffer stops accepting new data, in bytes.",
            source: "https://clouddocs.f5.com/api/irules/TCP__proxybufferhigh.html",
            examples: "when SERVER_CONNECTED {\n    log local0.debug \"[TCP::proxybufferhigh]\"\n}",
            return_value: "The proxy buffer high threshold.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: true,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::proxybufferhigh",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
