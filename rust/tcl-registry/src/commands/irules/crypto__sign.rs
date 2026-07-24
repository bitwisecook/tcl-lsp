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

//! `CRYPTO::sign` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::sign",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides a digital signature of a block of data.",
            synopsis: &[
                "CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
            ],
            snippet: "This iRules command is used to provide a digital signature of a block\nof data.\n\nCRYPTO::sign [-alg <>] [-ctx <> [-final]] [-key[hex] [<data>]\n\n     * Used to provide a digital signature of a block of data. Notes on\n       the flags:\n          + alg - algorithm. ASCII string from a given list (see below)\n            The spelling is lowercase and the iRule will fail for anything\n            not in the list. In ctx mode, alg must be given in the first\n            CRYPTO::command and cannot be modified.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__sign.html",
            examples: "set secret_key \"foobar1234\"\n\nset data \"This is my data\"\n\nset signed_data [CRYPTO::sign -alg hmac-sha1 -key $secret_key $data]\n\nif { [CRYPTO::verify -alg hmac-sha1 -key $secret_key -signature $signed_data $data] } {\n    log local0. \"Data verified\"\n}\n\nThe secret key will normally be some large string, size generally\ndictated by algorithm. The data is just whatever content you want to\nsign. The result of the CRYPTO::sign command will be a binary value, so",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-alg",
                    value: OptionValue::value("ALG"),
                    detail: "Signing algorithm.",
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
                OptionSpec {
                    name: "-key",
                    value: OptionValue::value("KEY"),
                    detail: "Binary key.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-keyhex",
                    value: OptionValue::value("KEY_HEX"),
                    detail: "Hex-encoded key.",
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
