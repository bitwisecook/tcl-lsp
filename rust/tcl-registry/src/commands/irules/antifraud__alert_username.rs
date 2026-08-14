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

//! `ANTIFRAUD::alert_username` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets username and for phishing also additional fields.",
            synopsis: &["ANTIFRAUD::alert_username (VALUE)?"],
            snippet: "ANTIFRAUD::alert_username ;\n                Returns username and for phishing also additional fields.\n\n            ANTIFRAUD::alert_username VALUE ;\n                Sets username and for phishing also additional fields.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_username.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert username: [ANTIFRAUD::alert_username].\"\n                ANTIFRAUD::alert_username new_value\n                log local0. \"new Alert username: [ANTIFRAUD::alert_username].\"\n            }",
            return_value: "ANTIFRAUD::alert_username ; Returns username and for phishing also additional fields.",
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
            synopsis: "ANTIFRAUD::alert_username (VALUE)?",
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
