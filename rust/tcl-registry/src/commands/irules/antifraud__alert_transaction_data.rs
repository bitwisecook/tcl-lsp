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

//! `ANTIFRAUD::alert_transaction_data` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_transaction_data",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets key-value list of all parameters marked to be attached.",
            synopsis: &["ANTIFRAUD::alert_transaction_data (VALUE)?"],
            snippet: "ANTIFRAUD::alert_transaction_data ;\n                Returns key-value list of all parameters marked to be attached.\n\n            ANTIFRAUD::alert_transaction_data VALUE ;\n                Sets key-value list of all parameters marked to be attached.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_transaction_data.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert transaction data: [ANTIFRAUD::alert_transaction_data].\"\n                ANTIFRAUD::alert_transaction_data new_value\n                log local0. \"new Alert transaction data: [ANTIFRAUD::alert_transaction_data].\"\n            }",
            return_value: "ANTIFRAUD::alert_transaction_data ; Returns key-value list of all parameters marked to be attached.",
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
            synopsis: "ANTIFRAUD::alert_transaction_data (VALUE)?",
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
