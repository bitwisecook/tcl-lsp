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

//! `ANTIFRAUD::username` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets username value, only in context of ANTIFRAUD_LOGIN event.",
            synopsis: &["ANTIFRAUD::username (USERNAME_ALIAS)?"],
            snippet: "ANTIFRAUD::username\n                Returns username value, only in context of ANTIFRAUD_LOGIN event.\n\n            ANTIFRAUD::username USERNAME_ALIAS ;\n                Replaces existing username with USERNAME_ALIAS, only in context of ANTIFRAUD_LOGIN event.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__username.html",
            examples: "when ANTIFRAUD_LOGIN {\n                log local0. \"Username [ANTIFRAUD::username] tried to log in.\"\n                ANTIFRAUD::username username_alias\n                log local0. \"Username alias is [ANTIFRAUD::username].\"\n            }",
            return_value: "ANTIFRAUD::username Returns username value, only in context of ANTIFRAUD_LOGIN event.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ANTIFRAUD"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ANTIFRAUD::username (USERNAME_ALIAS)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
