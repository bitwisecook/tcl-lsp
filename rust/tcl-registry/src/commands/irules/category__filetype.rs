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

//! `CATEGORY::filetype` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::filetype",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get mime type and mime subtype of payload.",
            synopsis: &["CATEGORY::filetype HTTP_PAYLOAD"],
            snippet: "Checks for the mime type and mime subtype of an HTTP request payload and returns the values to specified variables; use one or both to specify them name of the variable that you want the value to be given to.",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__filetype.html",
            examples: "when HTTP_RESPONSE {\n    # Collect 256 bytes of payload and trigger HTTP_RESPONSE_DATA\n    HTTP::collect 256\n}",
            return_value: "Sets the specified variables with the mime type or mime subtype",
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
            synopsis: "CATEGORY::filetype HTTP_PAYLOAD ?options?",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-mimetype",
                    value: OptionValue::value("TYPE"),
                    detail: "Variable name to store MIME type.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-mimesubtype",
                    value: OptionValue::value("SUBTYPE"),
                    detail: "Variable name to store MIME subtype.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
