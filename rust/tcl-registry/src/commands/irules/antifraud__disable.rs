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

//! `ANTIFRAUD::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables the anti-fraud plugin.",
            synopsis: &["ANTIFRAUD::disable"],
            snippet: "Disables the anti-fraud plugin.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable.html",
            examples: "when HTTP_REQUEST {\n                # Disable request with Antifraud-Disable header (bypass antifraud plugin)\n                if { [HTTP::header exists \"Antifraud-Disable\" ] } {\n                    ANTIFRAUD::disable\n                }\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ANTIFRAUD::disable",
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
