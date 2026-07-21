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

//! `DHCPv6::transaction_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::transaction_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns transaction id field from DHCPv6 message.",
            synopsis: &["DHCPv6::transaction_id"],
            snippet: "This command returns transaction id field from DHCPv6 message\n\nDetails (syntax):\nDHCPv6::transaction_id",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__transation_id.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Transaction_id [DHCPv6::transaction_id]\"\n    }",
            return_value: "This command returns transaction id field from DHCPv6 message",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv6::transaction_id",
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
