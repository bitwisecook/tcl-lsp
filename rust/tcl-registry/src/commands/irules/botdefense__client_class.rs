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

//! `BOTDEFENSE::client_class` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::client_class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the classification of the client based on the current request and its browsing history.",
            synopsis: &["BOTDEFENSE::client_class"],
            snippet: "Returns the classification of the client that sent the request. The returned value is one of the following strings:* unknown* browser* mobile_application* trusted_bot* untrusted_bot* malicious_bot* suspicious_browser. The command is similar to BOTDEFENSE::client_type but with higher resolution for bot classification: when BOTDEFENSE::client_type returns \"bot\", BOTDEFENSE::client_class returns the exact type of bot: malicious, trusted or untrusted.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__client_class.html",
            examples: "when BOTDEFENSE_ACTION {\n    log.local0. \"Client type after processing request: [BOTDEFENSE::client_class]\"\n}",
            return_value: "Returns the classification of the client that sent the request. When invoked in the BOTDEFENSE_REQUEST event it returns the type based on the previous requests of the same client, or \"unknown\" if the client is not recognized.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "BOTDEFENSE::client_class",
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
