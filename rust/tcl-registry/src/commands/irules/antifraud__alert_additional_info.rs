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

//! `ANTIFRAUD::alert_additional_info` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_additional_info",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
            synopsis: &["ANTIFRAUD::alert_additional_info (VALUE)?"],
            snippet: "ANTIFRAUD::alert_additional_info ;\n                Returns a list of keys and values that describes integrity parameters check failure or parameter values too long error.\n\n            ANTIFRAUD::alert_additional_info VALUE ;\n                Sets a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_additional_info.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert additional info: [ANTIFRAUD::alert_additional_info].\"\n                ANTIFRAUD::alert_additional_info new_value\n                log local0. \"new Alert additional info: [ANTIFRAUD::alert_additional_info].\"\n            }",
            return_value: "ANTIFRAUD::alert_additional_info ; Returns a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
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
            synopsis: "ANTIFRAUD::alert_additional_info (VALUE)?",
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
