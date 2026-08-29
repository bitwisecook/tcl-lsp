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

//! `ACCESS::respond` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::respond",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command generates new respond and automatically overrides the default respond.",
            synopsis: &[
                "ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ",
                "ACCESS::respond STATUS_CODE (((('content' | '-content') CONTENT)",
            ],
            snippet: "This command generates new respond and automatically overrides the\ndefault respond. This command only can be used only once per HTTP\nrequest, and subsequent calls to this command will return an error.\n\nHTTP iRules should be used with caution after an ACCESS::respond call.\nThey may not behave as expected since ACCESS::respond creates an HTTP response.\nAs of version 13.0.0, the way that HTTP caching interacts with the\nHTTP iRule commands has changed, so inconsistencies are expected when using\nHTTP iRules after ACCESS::respond.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__respond.html",
            examples: "when ACCESS_POLICY_COMPLETED {\n    set policy_result [ACCESS::policy result]\n    switch $policy_result {\n    \"allow\" {\n    # Do nothing\n    }\n    \"deny\" {\n        ACCESS::respond 401 content \"<html><body>Error: Failure in Authentication</body></html>\" Connection Close\n    }\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-ifile",
                    value: OptionValue::flag(),
                    detail: "Option -ifile.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-content",
                    value: OptionValue::value(""),
                    detail: "Option -content.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
