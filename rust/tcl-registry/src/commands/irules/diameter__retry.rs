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

//! `DIAMETER::retry` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Tries to send the Diameter message contained in the binary array \"binary_message\".",
            synopsis: &["DIAMETER::retry DIAMETER_MESSAGE (BOOL_ACROSS)?"],
            snippet: "This iRule command tries to send the Diameter message contained in the\nbinary array \"binary_message\".  This command, in conjunction with the\nDIAMETER::message command, can be used to write an iRule that will\nhold and retry messages.\n\nIf the optional argument \"across\" is specified as 1, the message will\nbe sent through the proxy and trigger the various iRule events.  If it\nis specified as 0, or not specified, the message will be sent directly\nand not experience any iRules, persistence, or other processing.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retry.html",
            examples: "when DIAMETER_EGRESS {\n   if { [DIAMETER::is_request] } {\n      set saved_message([DIAMETER::header hopid]) [DIAMETER::message]\n   }\n}",
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
            synopsis: "DIAMETER::retry DIAMETER_MESSAGE (BOOL_ACROSS)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
