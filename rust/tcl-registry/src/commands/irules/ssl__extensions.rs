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

//! `SSL::extensions` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::extensions",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or manipulates SSL extensions.",
            synopsis: &[
                "SSL::extensions (count |",
                "SSL::extensions insert OPAQUE_EXT",
            ],
            snippet: "Returns or manipulates SSL extensions.",
            source: "https://clouddocs.f5.com/api/irules/SSL__extensions.html",
            examples: "when CLIENTSSL_CLIENTHELLO {\n    set my_ext \"Hello world!\"\n    set my_ext_type 62965\n    SSL::extensions insert [binary format S1S1a* $my_ext_type [string length $my_ext] $my_ext]\n}",
            return_value: "SSL::extensions Returns the extensions sent by the peer as a single opaque byte array. Valid in all SSL handshake events (those other than *SSL_DATA).",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::extensions ?options?",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-index",
                    value: OptionValue::value("EXT_NUMBER"),
                    detail: "Return extension at specified index.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-type",
                    value: OptionValue::value("EXT_TYPE_VALUE"),
                    detail: "Return extension matching specified type value.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
