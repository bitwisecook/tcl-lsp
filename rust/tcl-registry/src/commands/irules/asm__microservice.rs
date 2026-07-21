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

//! `ASM::microservice` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::microservice",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "request matched microservice",
            synopsis: &["ASM::microservice"],
            snippet: "returns the microservice matched for the request;",
            source: "https://clouddocs.f5.com/api/irules/ASM__microservice.html",
            examples: "when ASM_REQUEST_DONE \n            {\n\t\tif {[ASM::microservice] eq \"*a/login.php\"}\n\t\t{\n\t\t\tlog local0. \"Microservice : found\"\n\t        }\n            }",
            return_value: "returns the microservice matched for the request;",
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
            synopsis: "ASM::microservice",
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
