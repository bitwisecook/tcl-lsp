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

//! `CRYPTO::hash` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::hash",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Generates a hash on a piece of data.",
            synopsis: &[
                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
            ],
            snippet: "This iRules command generates a hash on a piece of data\n\nCRYPTO::hash [-alg <>] [-ctx <> [-final]] [<data>]\n\n     * Generates a hash on a piece of data\n\nAlgorithm List\n\n     * md5\n     * ripemd160\n     * sha1\n     * sha224\n     * sha256\n     * sha384\n     * sha512",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__hash.html",
            examples: "when HTTP_REQUEST {\nif {[class match [b64encode [CRYPTO::hash -alg sha384 [HTTP::host][HTTP::path]]] equals HASH ]} {\n    log local0. \" this FQDN + PATH is mathing - [HTTP::host][HTTP::path]\"\n}\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-alg",
                    value: OptionValue::value("ALG"),
                    detail: "Hash algorithm.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-ctx",
                    value: OptionValue::value("CTX_VAR"),
                    detail: "Context variable for multi-step operations.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-final",
                    value: OptionValue::flag(),
                    detail: "Finalize context-based operation.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
