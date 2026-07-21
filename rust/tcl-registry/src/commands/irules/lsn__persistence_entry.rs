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

//! `LSN::persistence-entry` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::persistence-entry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Create or lookup LSN translation address.",
            synopsis: &[
                "LSN::persistence-entry (delete|get) CLIENT_ADDR",
                "LSN::persistence-entry create (-override)? LSN_POOL CLIENT_ADDR TRANSLATION_ADDR (TIMEOUT)?",
            ],
            snippet: "Create or lookup LSN translation address. Those commands are linked to CGNAT module introduced in 11.3. You need to license and provision this module to use this command.\n\nLSN::persistence-entry create [-override] <client_address>[:<client_port>] [<translation_address>[:<translation_port>]]\nLSN::persistence-entry get <client_address>[:<client_port>]\n\nv11.4+\nLSN::persistence-entry create [-override] <lsn_pool>  <client_address>[:<port>] <translation_address>[:<port>]]  [timeout]\n\nv11.5+\nLSN::persistence-entry delete <client_address>",
            source: "https://clouddocs.f5.com/api/irules/LSN__persistence-entry.html",
            examples: "when CLIENT_ACCEPTED {\n    set clientIP [IP::client_addr]\n}",
            return_value: "LSN::persistence-entry create",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LSN::persistence-entry (delete|get) CLIENT_ADDR",
        }],
        options: const {
            &[OptionSpec {
                name: "-override",
                value: OptionValue::flag(),
                detail: "Option -override.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
