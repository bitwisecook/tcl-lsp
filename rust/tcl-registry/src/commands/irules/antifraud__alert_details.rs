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

//! `ANTIFRAUD::alert_details` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_details",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets alert details.",
            synopsis: &["ANTIFRAUD::alert_details (VALUE)?"],
            snippet: "ANTIFRAUD::alert_details ;\n                Returns alert details.\n\n            ANTIFRAUD::alert_details VALUE ;\n                Sets alert details.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_details.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert details: [ANTIFRAUD::alert_details].\"\n                ANTIFRAUD::alert_details new_value\n                log local0. \"new Alert details: [ANTIFRAUD::alert_details].\"\n            }",
            return_value: "ANTIFRAUD::alert_details ; Returns alert details.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ANTIFRAUD"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ANTIFRAUD::alert_details (VALUE)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
