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

//! `domain` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "domain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Parses the specified string as a dotted domain name and returns the last portions of the domain name.",
            synopsis: &["domain DOMAIN COUNT"],
            snippet: "A custom iRule function which parses the specified string as a\ndotted domain name and returns the last <count> portions of the domain\nname.",
            source: "https://clouddocs.f5.com/api/irules/domain.html",
            examples: "when HTTP_REQUEST\nif { [HTTP::uri] ends_with \".html\" } {\n      pool cache_pool\n      set key [crc32 [concat [domain [HTTP::host] 2] [HTTP::uri]]]\n}\n...\n\nThis code:\n\n log local0. [domain www.sub.my.domain.com 1]   ; # result: com\n log local0. [domain www.sub.my.domain.com 2]   ; # result: domain.com\n log local0. [domain www.sub.my.domain.com 3]   ; # result: my.domain.com\n log local0. [domain www.sub.my.domain.com 4]   ; # result: sub.my.domain.com",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "domain DOMAIN COUNT",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
