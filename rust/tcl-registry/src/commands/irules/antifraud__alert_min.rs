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

//! `ANTIFRAUD::alert_min` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_min",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets variable data from client side, e.g.",
            synopsis: &["ANTIFRAUD::alert_min (VALUE)?"],
            snippet: "ANTIFRAUD::alert_min ;\n                Returns variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.\n\n            ANTIFRAUD::alert_min VALUE ;\n                Sets variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_min.html",
            examples: "when ANTIFRAUD_ALERT {\n                if {[ANTIFRAUD::alert_type] eq \"js_vhtml\"} {\n                    if {[ANTIFRAUD::alert_component] eq \"external_sources\"} {\n                        log local0. \"Alert forbidden added element: [ANTIFRAUD::alert_min]\"\n                    }\n                    elseif {[ANTIFRAUD::alert_component] eq \"trojan_bait\"} {\n                        log local0. \"Alert bait signatures: [ANTIFRAUD::alert_min]\"\n                    }\n                }",
            return_value: "ANTIFRAUD::alert_min ; Returns variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.",
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
            synopsis: "ANTIFRAUD::alert_min (VALUE)?",
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
