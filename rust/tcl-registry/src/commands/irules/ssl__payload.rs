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

//! `SSL::payload` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// The command's subcommands; any other first word is an offset or length.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "replace",
        arity: Arity::exact(3),
        detail: "Replace bytes in collected plaintext payload.",
        synopsis: "SSL::payload replace <offset> <length> <data>",
        mutator: true,
        // A negative offset is refused at load with `out of expected
        // range(0 - 4294967295)` (BIG-IP 21.1.0.1,
        // `evidence/tcl/bigip-ssl-payload-replace-negative.md`).
        arg_domains: &[(0, IntegerDomain::Range(0, 4_294_967_295))],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::exact(0),
        detail: "Returns the amount of collected plaintext data in bytes.",
        synopsis: "SSL::payload length",
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::payload",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::new(0, 4),
        data_collection: Some(SSL_PAYLOAD),
        default_form_first_word: Some(DefaultFormFirstWord::Integer),
        hover: Some(HoverSnippet {
            summary: "Returns and manipulates plaintext data collected via SSL::collect.",
            synopsis: &[
                "SSL::payload ?<offset>? ?<length>?",
                "SSL::payload replace <offset> <length> <data>",
                "SSL::payload length",
            ],
            snippet: "The SSL::payload commands allow you to return and manipulate the data collected via the SSL::collect command. This data is in plaintext format.",
            source: "https://clouddocs.f5.com/api/irules/SSL__payload.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0. \"[IP::client_addr]:[TCP::client_port]: SSL handshake completed, collecting SSL payload\"\n    SSL::collect\n}",
            return_value: "SSL::payload length Returns the amount of plaintext data collected by the SSL::collect command.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "SSL::payload ?<offset>? ?<length>?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }]),
        taint_source: Some(TaintColour::TAINTED),
        // As `TCP::payload` (BIG-IP 21.1.0.1, `evidence/tcl/bigip-ssl-payload-release-negative.md`).
        arg_domains: &[
            (0, IntegerDomain::Range(0, i64::MAX)),
            (1, IntegerDomain::Range(0, i64::MAX)),
        ],
        ..CommandSpec::DEFAULT
    }
}
