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

//! `ANTIFRAUD::enable` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::enable",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables the anti-fraud plugin.",
            synopsis: &["ANTIFRAUD::enable (ANTIFRAUD_PROFILE)?"],
            snippet: "Enables the anti-fraud plugin.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__enable.html",
            examples: "when HTTP_REQUEST {\n                # apply default anti-fraud profile on the transaction with Antifraud-Foo HTTP header\n                if { [HTTP::header exists \"Antifraud-Foo\" ] } {\n                    ANTIFRAUD::enable\n                }\n                # apply /Common/antifraud_bar profile on the transaction with Antifraud-Bar HTTP header\n                if { [HTTP::header exists \"Antifraud-Bar\" ] } {\n                    ANTIFRAUD::enable /Common/antifraud_bar\n                }\n            }",
            return_value: "ANTIFRAUD::enable Applies the default anti-fraud profile attached to the virtual server.",
        }),
        forms: &[FormSpec {
            synopsis: "ANTIFRAUD::enable (ANTIFRAUD_PROFILE)?",
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
