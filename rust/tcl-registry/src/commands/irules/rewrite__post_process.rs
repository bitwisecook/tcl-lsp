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

//! `REWRITE::post_process` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::post_process",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggle post processing functionality.",
            synopsis: &["REWRITE::post_process (SWITCH)?"],
            snippet: "When REWRITE::post_process is called (without any arguments), it\nwill return a \"0\" to signify that it is off, or an \"1\" to signify that\nit is on. By default, it is off. Use the command \"REWRITE::post_process\n1\" to turn on the post process functionality and \"REWRITE::post_process\n0\" to turn it off. When post_process is on, the\nREWRITE_RESPONSE_DONE event is triggered. Otherwise, the\nREWRITE_RESPONSE_DONE event is ignored.",
            source: "https://clouddocs.f5.com/api/irules/REWRITE__post_process.html",
            examples: "when REWRITE_REQUEST_DONE {\n  if { \"[HTTP::host][HTTP::path]\" eq \"www.external.com/contents.php\" } {\n    # Found the file we wanted to modify\n    REWRITE::post_process 1\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["REWRITE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "REWRITE::post_process (SWITCH)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
