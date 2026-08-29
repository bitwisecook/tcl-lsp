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

//! `AUTH::wantcredential_prompt_style` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_prompt_style",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns an authorization session authidXs credential prompt style.",
            synopsis: &["AUTH::wantcredential_prompt_style AUTH_ID"],
            snippet: "Returns the authorization session authid’s credential prompt style that\nthe system last requested (when the system generated an\nAUTH_WANTCREDENTIAL event). The value of the <authid> argument is\neither echo_on, echo_off, or unknown. This command is especially\nhelpful in providing authentication services to interactive protocols\n(or example, telnet and ftp), where the actual text prompts and\nresponses may be directly communicated with the remote user.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_prompt_style.html",
            examples: "when AUTH_WANTCREDENTIAL {\n  HTTP::respond 401 \"WWW-Authenticate\" \"Basic realm=\\\"\\\"\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::wantcredential_prompt_style AUTH_ID",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
