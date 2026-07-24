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

//! `ANTIFRAUD::disable_auto_transactions` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_auto_transactions",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables automatic transactions for the current transaction.",
            synopsis: &["ANTIFRAUD::disable_auto_transactions"],
            snippet: "Disables automatic transactions for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_auto_transactions.html",
            examples: "when HTTP_REQUEST {\n                if { [HTTP::header exists \"Antifraud-Disable-AutoTransactions\" ] } {\n                    ANTIFRAUD::disable_auto_transactions\n                    log local0. \"Automatic Transactions disabled\"\n                }\n            }",
            return_value: "Disables automatic transactions for the current transaction.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ANTIFRAUD::disable_auto_transactions",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
