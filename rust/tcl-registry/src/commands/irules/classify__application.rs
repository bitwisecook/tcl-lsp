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

//! `CLASSIFY::application` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::application",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows to set or add an app name to the classification.",
            synopsis: &["CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME"],
            snippet: "This command allows you to set or add an app name to the\nclassification.\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFY::application set <app_name>\n\n     * will immediately classify flow as app_name. The classification by\n       the classification engine will be bypassed. The category classification token this app_name belongs to will also be attached.\n\nCLASSIFY::application add <app_name>\n\n     * adds an application classification token to the final\n       classification result issued by the classification engine.",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__application.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &["CLIENT_ACCEPTED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME",
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
