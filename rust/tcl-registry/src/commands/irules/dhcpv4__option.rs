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

//! `DHCPv4::option` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::option",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command retrieves,sets or deletes the option by id number.",
            synopsis: &["DHCPv4::option (delete)? OPTION (VALUE)?"],
            snippet: "This command retrieves,sets or deletes the option by id number\n\nDetails (syntax);\nDHCPv4::option <id>\nDHCPv4::option <id> <value>\nDHCPv4::option delete <id>",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__option.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Option [DHCPv4::option 18]\"\n    }",
            return_value: "This command returns value by option id number when retrieving",
        }),
        forms: &[FormSpec {
            synopsis: "DHCPv4::option (delete)? OPTION (VALUE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
