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

//! `HTTP::has_responded` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::has_responded",
        traits: Traits::PURE,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Returns true if this HTTP transaction has been prematurely completed by an iRule command or other filter logic.",
            synopsis: &["HTTP::has_responded"],
            snippet: "This can be triggered by HTTP::respond, HTTP::redirect, HTTP::retry, and some ACCESS commands.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__has_responded.html",
            examples: "when HTTP_REQUEST {\n  # Used for cases where only one response to the client is permitted.\n  # Another HTTP::respond might have been called in other iRULE script.\n  if {[HTTP::has_responded]} {\n    log local0. \"Have already responded.\"\n  } else {\n    HTTP::respond 200 content {<html><body>First and Only Response</body></html>}\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "HTTP::has_responded",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ResponseCommit,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
