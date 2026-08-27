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

//! `ICAP::method` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the ICAP request method.",
            synopsis: &["ICAP::method"],
            snippet: "The ICAP::method command returns the ICAP request method.\nThis will either be \"REQMOD\" or \"RESPMOD\"",
            source: "https://clouddocs.f5.com/api/irules/ICAP__method.html",
            examples: "when ICAP_REQUEST {\n                if {[ICAP::method] == \"REQMOD\"} {\n                    ICAP::header add X-Request $seqno\n                }\n            }",
            return_value: "Returns the ICAP request method.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ICAP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ICAP::method",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
