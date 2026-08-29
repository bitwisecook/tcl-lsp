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

//! `CLASSIFY::urlcat` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::urlcat",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows to set or add an url category to the classification.",
            synopsis: &["CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME"],
            snippet: "This command allows you to set or add an url category to the\nclassification.\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFY::urlcat set <URL_Category>\n\n     * will immediately classify flow as URL_category.\n\nCLASSIFY::application add <app_name>\n\n     * adds an URL Category to the URL classification token to the final\n       classification result issued by the classification engine. This can\n       be issued multiple times in order to add multiple tokens to the classification result.",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__urlcat.html",
            examples: "when HTTP_REQUEST\n{\n    if { [HTTP::host] contains \"google\"} {\n        CLASSIFY::urlcat set customCategory\n    }\n}",
            return_value: "",
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
            synopsis: "CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
