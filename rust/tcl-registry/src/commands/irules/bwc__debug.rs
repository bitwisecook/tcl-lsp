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

//! `BWC::debug` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::debug",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command is used for troubleshooting a bwc policy instance.",
            synopsis: &["BWC::debug ('start')", "BWC::debug ('stop')"],
            snippet: "This command enables debug logs on per policy instance. However the bwc sys db variables for bwc trace need to be enabled and appropriate levels needs to be set as required.",
            source: "https://clouddocs.f5.com/api/irules/BWC__debug.html",
            examples: "when CLIENT_ACCEPTED {\n    set mycookie [IP::remote_addr]:[TCP::remote_port]\n    BWC::policy attach test_pol $mycookie\n    log  local0. \"BWC::policy attach  $mycookie\"\n    BWC::debug start session\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "BWC::debug ('start')",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
