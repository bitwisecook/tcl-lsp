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

//! `DHCPv6::option` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::option",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command retrieves, sets or deletes the option by id.",
            synopsis: &["DHCPv6::option (delete)? OPTION (VALUE)?"],
            snippet: "This command retrieves, sets or deletes the option by id\n\nDetails (syntax);\nDHCPv6::option <id>\nDHCPv6::option <id> <value>\nDHCPv6::option delete <id>",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__option.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Option [DHCPv6::option 18]\"\n    }",
            return_value: "when retrieving, this command returns the value of the option via option id",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv6::option (delete)? OPTION (VALUE)?",
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
