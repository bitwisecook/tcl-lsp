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

//! `ANTIFRAUD::alert_expected_value` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_expected_value",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets expected (verified) value, for example in strong integrity check.",
            synopsis: &["ANTIFRAUD::alert_expected_value (VALUE)?"],
            snippet: "ANTIFRAUD::alert_expected_value ;\n                Returns expected (verified) value, for example in strong integrity check.\n\n            ANTIFRAUD::alert_expected_value VALUE ;\n                Sets expected (verified) value, for example in strong integrity check.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_expected_value.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert expected value: [ANTIFRAUD::alert_expected_value].\"\n                ANTIFRAUD::alert_expected_value new_value\n                log local0. \"new Alert expected value: [ANTIFRAUD::alert_expected_value].\"\n            }",
            return_value: "ANTIFRAUD::alert_expected_value ; Returns expected (verified) value, for example in strong integrity check.",
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
            synopsis: "ANTIFRAUD::alert_expected_value (VALUE)?",
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
