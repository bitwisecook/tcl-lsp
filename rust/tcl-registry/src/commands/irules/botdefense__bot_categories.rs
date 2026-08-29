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

//! `BOTDEFENSE::bot_categories` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_categories",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the list of category names to which the current client belongs.",
            synopsis: &["BOTDEFENSE::bot_categories"],
            snippet: "Returns the list of category names to which the current client belongs. These categories are determined by the anomalies found for the respective client. Note these categories are additional to the bot signature category which is applicable if a bot signature was found.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_categories.html",
            examples: "when BOTDEFENSE_ACTION {\n    foreach {cat} [BOTDEFENSE::bot_categories] {\n        log.local0. \"Found category: $cat\"\n    }\n}",
            return_value: "Returns a list of all category names to which the current client belongs based on the anomalies found for the client. The categories come in addition to the bot signature category optionally detected and returned in BOTDEFENSE::bot_signature_category. If no anomaly found then the list will be empty.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "BOTDEFENSE::bot_categories",
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
