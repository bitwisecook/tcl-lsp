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

//! `MR::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Access data collected using MR::collect command.",
            synopsis: &["MR::payload ( 'length' )?"],
            snippet: "This command can be used to access payload collected using the COLLECT command.\n\nSYNTAX\n\nMR::payload [length]\n\nMR::payload\n    Returns the collected payload obtained as a result of a prior call to MR::collect.\n\nMR::payload length\n    Returns the length of payload of a MR message.",
            source: "https://clouddocs.f5.com/api/irules/MR__payload.html",
            examples: "when MR_DATA {\n                log local0 \"Payload: [MR::payload]\"\n            }",
            return_value: "When called without an argument, this command returns the collected payload of an MR message.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "MR::payload ( 'length' )?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        data_collection: Some(MR_PAYLOAD),
        ..CommandSpec::DEFAULT
    }
}
