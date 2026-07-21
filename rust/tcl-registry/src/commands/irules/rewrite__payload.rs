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

//! `REWRITE::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries for or manipulates REWRITE payload.",
            synopsis: &[
                "REWRITE::payload (LENGTH | (OFFSET LENGTH))?",
                "REWRITE::payload length",
                "REWRITE::payload replace OFFSET LENGTH PAYLOAD",
            ],
            snippet: "Queries for or manipulates REWRITE payload (content) information. With\nthis command, you can retrieve content, query for content size, or\nreplace a certain amount of content.",
            source: "https://clouddocs.f5.com/api/irules/REWRITE__payload.html",
            examples: "when REWRITE_RESPONSE_DONE {\n    # The rewrite_response_done event isn't absolutely necessary because browser will just ignore any html tags that it doesn't recongnize.\n    # However, it will be cleaner if we remove it nevertheless\n\n    set data [REWRITE::payload]\n    # Find the tags we inserted\n    set start [string first {<apm_do_not_touch>} $data]\n    set end [string last {</apm_do_not_touch>} $data]\n    # Determines the amount of characters to remove",
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
            synopsis: "REWRITE::payload (LENGTH | (OFFSET LENGTH))?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
