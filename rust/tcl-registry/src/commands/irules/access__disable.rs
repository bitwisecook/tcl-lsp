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

//! `ACCESS::disable` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::disable",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Control enforcement for a particular request URI.",
            synopsis: &["ACCESS::disable"],
            snippet: "This command disables the access control enforcement for a particular\nrequest URI. The request is passed through access control module\nwithout any access control checks (excludes valid session check as well\nas policy allowed check).\n\nACCESS::disable\n\n     * Disable the access control enforcement for a particular request\n       URI.\n\n * Requires APM module",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__disable.html",
            examples: "when HTTP_REQUEST {\n\n       # Check the requested HTTP path\n       switch -glob [string tolower [HTTP::path]] {\n              \"/apm_uri1*\" -\n              \"/apm_uri2*\" -\n              \"/apm_uri3*\" {\n                     # Enable APM for these paths\n                     ACCESS::enable\n              }\n              default {\n                     # Disable APM for all other paths\n                     ACCESS::disable\n              }\n       }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::disable",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
