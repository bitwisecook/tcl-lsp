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

//! `imid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "imid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns an i-mode identifier string.",
            synopsis: &["imid"],
            snippet: "Parses the BIG-IP 4.X http_uri variable and the user-agent header field to return an i-mode identifier string that can be used for i-mode session persistence. This is a BIG-IP 4.X function, provided for backward compatibility.\n\nThe imid function takes no arguments and simply returns the string representing the i-mode identifier or the empty string, if none is found.",
            source: "https://clouddocs.f5.com/api/irules/imid.html",
            examples: "",
            return_value: "Parses the BIG-IP 4.X http_uri variable and the '''user-agent* header field to return an i-mode identifier string that can be used for i-mode session persistence.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "imid",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("IMID::id"),
        ..CommandSpec::DEFAULT
    }
}
