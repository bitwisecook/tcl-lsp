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

//! `ANTIFRAUD::alert_view_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_view_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the configured URL and view which triggered this alert.",
            synopsis: &["ANTIFRAUD::alert_view_id (VALUE)?"],
            snippet: "ANTIFRAUD::alert_view_id ;\n                Returns the configured URL and view which triggered this alert. Empty if not a view.\n\n            ANTIFRAUD::alert_view_id VALUE ;\n                Sets the configured URL and view which triggered this alert. Empty if not a view.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_view_id.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert View ID: [ANTIFRAUD::alert_view_id].\"\n                ANTIFRAUD::alert_view_id new_value\n                log local0. \"new Alert View ID: [ANTIFRAUD::alert_view_id].\"\n            }",
            return_value: "ANTIFRAUD::alert_view_id ; Returns the configured URL and view which triggered this alert. Empty if not a view.",
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
            synopsis: "ANTIFRAUD::alert_view_id (VALUE)?",
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
