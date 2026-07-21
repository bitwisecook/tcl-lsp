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

//! `FIX::tag` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FIX::tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Defines/deletes the mapping between senderCompID and a tag map data group.",
            synopsis: &[
                "FIX::tag map set SENDER DATA_GROUP",
                "FIX::tag map delete",
                "FIX::tag get TAG",
            ],
            snippet: "This command can either retrieve tag value or update the mapping\nbetween senderCompID and a tag map data group. In latter case If a\nmapping is already defined in the profile attributes for\nsender-tag-map, it is overwritten by the iRule mapping.",
            source: "https://clouddocs.f5.com/api/irules/FIX__tag.html",
            examples: "when RULE_INIT {\n  # with the follow command, tag 10001 is replaced to 20001 for the messages sent by client_1\n  # before sending to pool member and reverse-replaced(20001 to 10001) to client_1\n  FIX::tag map set client_1 data_group_1\n  FIX::tag map set client_2 data_group_1\n  FIX::tag map set client_3 data_group_2\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FIX"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "FIX::tag map set SENDER DATA_GROUP",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
