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

//! `ANTIFRAUD::enable_log` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::enable_log",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables Anti-Fraud TMM logs for the current transaction.",
            synopsis: &["ANTIFRAUD::enable_log (LOG_LEVEL)?"],
            snippet: "ANTIFRAUD::enable_log\n                Enables Anti-Fraud TMM logs at 'Informational' (default) log level for the current transaction.\n\n            ANTIFRAUD::enable_log LOG_LEVEL ;\n                Enables Anti-Fraud TMM logs at 'LOG_LEVEL' (can be any of: 'Error'/'Warning'/'Notice'/'Informational'/'Debug') log level for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__enable_log.html",
            examples: "when HTTP_REQUEST {\n                if { [HTTP::header exists \"Antifraud-Enable-log\" ] } {\n                    ANTIFRAUD::enable_log\n                    log local0. \"Logs enabled\"\n                }\n            }",
            return_value: "ANTIFRAUD::enable_log No return value (enables Anti-Fraud TMM logs at default log level for the current transaction).",
        }),
        forms: &[FormSpec {
            synopsis: "ANTIFRAUD::enable_log (LOG_LEVEL)?",
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
