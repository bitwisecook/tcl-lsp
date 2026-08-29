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

//! `ANTIFRAUD::alert_transaction_id` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_transaction_id",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets alert HTTP transaction ID.",
            synopsis: &["ANTIFRAUD::alert_transaction_id (VALUE)?"],
            snippet: "ANTIFRAUD::alert_transaction_id ;\n                Returns alert HTTP transaction ID.\n\n            ANTIFRAUD::alert_transaction_id VALUE ;\n                Sets alert HTTP transaction ID.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_transaction_id.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert transaction ID: [ANTIFRAUD::alert_transaction_id].\"\n                ANTIFRAUD::alert_transaction_id new_value\n                log local0. \"new Alert transaction ID: [ANTIFRAUD::alert_transaction_id].\"\n            }",
            return_value: "ANTIFRAUD::alert_transaction_id ; Returns alert HTTP transaction ID.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ANTIFRAUD"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ANTIFRAUD::alert_transaction_id (VALUE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
