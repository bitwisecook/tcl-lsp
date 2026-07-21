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

//! `GENERICMESSAGE::message` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets values for messages in the generic message profile.",
            synopsis: &[
                "GENERICMESSAGE::message (len | length)",
                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                "GENERICMESSAGE::message is_request (BOOLEAN)?",
                "GENERICMESSAGE::message data (DATA)?",
            ],
            snippet: "The GENERICMESSAGE::message command returns or sets values from\nthe current message being processed by the generic message profile.",
            source: "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__message.html",
            examples: "when GENERICMESSAGE_INGRESS {\n    GENERICMESSAGE::message src us\n    GENERICMESSAGE::message dst them\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["GENERICMSG", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GENERICMESSAGE::message (len | length)",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
