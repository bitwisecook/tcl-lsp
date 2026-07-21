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

//! `ASM::captcha` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::captcha",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Responds to the client with a CAPTCHA challenge.",
            synopsis: &["ASM::captcha"],
            snippet: "Responds to the client with a CAPTCHA challenge. \n            Note although ASM will send the CAPTCHA challenge screen back to the user, the enforcement is not always done automatically. \n            To enforce the correct CAPTCHA response, the ASM::captcha_status command should be used.",
            source: "https://clouddocs.f5.com/api/irules/ASM__captcha.html",
            examples: "le counts the number of violations, and if it exceeds 3,\n            # it issues a CAPTCHA action.\n            when ASM_REQUEST_DONE {\n                if {[ASM::violation count] > 3 and [ASM::severity] eq \"Error\"} {\n                    ASM::captcha\n                }\n            }",
            return_value: "Returns a string signifying if the challenge was sent successfully: \"ok\" - CAPTCHA challenge was sent successfully \"nok asm blocked request\" - CAPTCHA challenge was not sent, because a blocking page action was performed \"nok asm uncaptcha command was raised\" - CAPTCHA challenge was not sent, because…",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::captcha",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
