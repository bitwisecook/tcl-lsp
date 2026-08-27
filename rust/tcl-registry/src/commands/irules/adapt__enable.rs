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

//! `ADAPT::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables, disables or returns the enable state.",
            synopsis: &["ADAPT::enable (ADAPT_CTX)? (ADAPT_SIDE)? (BOOLEAN)?"],
            snippet: "The ADAPT::enable command enables, disables or returns the enable\nstate of the ADAPT filter on the current or specified side of the\nvirtual server connection for which the iRule is being executed.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__enable.html",
            examples: "when HTTP_REQUEST {\n     ADAPT::enable true\n     ADAPT::enable response false\n}",
            return_value: "Returns the current of modified enable state.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ADAPT::enable (ADAPT_CTX)? (ADAPT_SIDE)? (BOOLEAN)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
