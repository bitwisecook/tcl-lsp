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

//! `SIP::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the accumulated SIP data content.",
            synopsis: &[
                "SIP::payload (LENGTH | (OFFSET LENGTH))?",
                "SIP::payload length",
                "SIP::payload replace OFFSET LENGTH PAYLOAD",
                "SIP::payload insert OFFSET PAYLOAD",
            ],
            snippet: "Returns the accumulated SIP data content.",
            source: "https://clouddocs.f5.com/api/irules/SIP__payload.html",
            examples: "when SIP_REQUEST {\n    log local0. \"unmodified request [SIP::payload]\"\n}",
            return_value: "Returns the SIP data accumulated so far",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["MR_EGRESS", "MR_FAILED", "MR_INGRESS"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SIP::payload (LENGTH | (OFFSET LENGTH))?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
