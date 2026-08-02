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

//! `SSL::unclean_shutdown` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::unclean_shutdown",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the value of the Unclean Shutdown setting.",
            synopsis: &["SSL::unclean_shutdown (enable | disable)"],
            snippet: "Sets the value of the Unclean Shutdown setting. This command only affects the current connection, and only affects the current context (e.g., when run in a client-side context, it only affects the current client-side connection).",
            source: "https://clouddocs.f5.com/api/irules/SSL__unclean_shutdown.html",
            examples: "# Note that for this iRule, unclean shutdown should be disabled in the clientssl profile\nwhen HTTP_REQUEST {\n    if { [HTTP::header \"User-Agent\"] contains \"MSIE\" } {\n        SSL::unclean_shutdown enable\n    }\n}",
            return_value: "SSL::unclean_shutdown <\"enable\" | \"disable\"> sets the current client-side or server-side SSL connection's Unclean Shutdown setting.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::unclean_shutdown (enable | disable)",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
