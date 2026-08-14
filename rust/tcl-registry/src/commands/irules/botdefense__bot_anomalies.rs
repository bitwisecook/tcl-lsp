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

//! `BOTDEFENSE::bot_anomalies` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_anomalies",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the list of names of anomalies detected for the client that sent the current request.",
            synopsis: &["BOTDEFENSE::bot_anomalies"],
            snippet: "Returns the list of names of anomalies detected for the client that sent the current request. Some anomalies may have been detected in previous requests of the same client and are still valid.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_anomalies.html",
            examples: "when BOTDEFENSE_ACTION {\n    foreach {anomaly} [BOTDEFENSE::bot_anomalies] {\n        log.local0. \"Found anomaly: $anomaly\"\n    }\n}",
            return_value: "Returns a list of names of all anomalies detected for the sending client. In case no anomalies found it returns an empty list.",
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
            synopsis: "BOTDEFENSE::bot_anomalies",
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
