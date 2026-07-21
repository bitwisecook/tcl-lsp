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

//! `DIAMETER::persist` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the persistence key being used for the current message.",
            synopsis: &[
                "DIAMETER::persist",
                "DIAMETER::persist reset",
                "DIAMETER::persist use",
            ],
            snippet: "This iRule command returns the persistence key being used for the\ncurrent message. If new persist key is provided, the existing\npersistence key will be replaced. The value of the new key MUST be the\nvalue of a valid AVP in the message. An AVP attribute name should not\nbe given as the new key value.\n\nIf bidirection is specified as false, disable(d), no, 0, or is\nunspecified, then persistence is not bidirectional. If bidirection is\nspecified as true, enable(d), yes, or 1 this persistence entry is\nbidirectional.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__persist.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message, persistence key is [DIAMETER::persist]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::persist",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
