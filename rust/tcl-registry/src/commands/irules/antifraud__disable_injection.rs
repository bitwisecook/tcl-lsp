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

//! `ANTIFRAUD::disable_injection` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_injection",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables Anti-Fraud injections for the current transaction.",
            synopsis: &["ANTIFRAUD::disable_injection"],
            snippet: "Disables Anti-Fraud injections for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_injection.html",
            examples: "when HTTP_RESPONSE {\n                if { [HTTP::header exists \"Antifraud-Disable-Injection\" ] } {\n                    ANTIFRAUD::disable_injection\n                    log local0. \"Injections disabled\"\n                }\n            }",
            return_value: "Disables Anti-Fraud injections for the current transaction.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ANTIFRAUD::disable_injection",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
