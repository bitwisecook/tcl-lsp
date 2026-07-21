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

//! `HSL::send` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HSL::send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sends data via High Speed Logging.",
            synopsis: &["HSL::send HANDLE DATA"],
            snippet: "Send data via High Speed Logging",
            source: "https://clouddocs.f5.com/api/irules/HSL__send.html",
            examples: "when CLIENT_ACCEPTED {\n    set hsl [HSL::open -proto UDP -pool syslog_server_pool]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HSL::send HANDLE DATA",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        arg_roles: &[(0, ArgRole::Channel)],
        ..CommandSpec::DEFAULT
    }
}
