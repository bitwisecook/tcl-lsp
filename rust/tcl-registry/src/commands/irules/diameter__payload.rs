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

//! `DIAMETER::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets DIAMETER message payload.",
            synopsis: &["DIAMETER::payload ('replace' PAYLOAD)?"],
            snippet: "This iRule command gets or sets the current DIAMETER message\\'s\npayload, as a byte string.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__payload.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message, with payload [DIAMETER::payload]\"\n}",
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
            synopsis: "DIAMETER::payload ('replace' PAYLOAD)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        byte_array_payload: Some(BytePayloadSpec {
            replace_data_index: 1,
            ..BytePayloadSpec::DEFAULT
        }),
        data_collection: Some(DIAMETER_PAYLOAD),
        ..CommandSpec::DEFAULT
    }
}
