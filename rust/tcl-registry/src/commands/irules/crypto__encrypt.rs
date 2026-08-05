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

//! `CRYPTO::encrypt` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::encrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This iRules command encrypts data.",
            synopsis: &["CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )"],
            snippet: "This iRules command encrypts data. A ciphertext encrypted with this\ncommand should be decryptable by third party software.\n\nCRYPTO::encrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n                [-padding <\"pkcs\" | \"oaep\" | \"none\">]\n\n     * encrypts data based on several parameters\n          + alg - algorithm. ASCII string from a given list (see below)\n            The spelling is lowercase and the iRule will fail for anything\n            not in the list. In ctx mode, alg must be given in the first\n            CRYPTO:: command and cannot be modified.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__encrypt.html",
            examples: "Encrypt an MSISDN header\n# Encrypt the MSISDN header for each request.\n# The encryption is deliberately designed to be insecure;\n# that is, the same MSISDN will always be encrypted to\n# the same ciphertext. And since the IV will always be\n# the same for each encryption, there's no need to send\n# it out with the ciphertext.\n#\nwhen SIP_REQUEST {\n    set key \"abed1ddc04fbb05856bca4a0ca60f21e\"\n    set iv \"d78d86d9084eb9239694c9a733904037\"",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-alg",
                    value: OptionValue::value("ALG"),
                    detail: "Encryption algorithm.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-ctx",
                    value: OptionValue::value("CTX_VAR"),
                    detail: "Context variable for multi-step operations.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-final",
                    value: OptionValue::flag(),
                    detail: "Finalize context-based operation.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-key",
                    value: OptionValue::value("KEY"),
                    detail: "Binary key.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-keyhex",
                    value: OptionValue::value("KEY_HEX"),
                    detail: "Hex-encoded key.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-iv",
                    value: OptionValue::value("IV"),
                    detail: "Initialization vector (binary).",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-ivhex",
                    value: OptionValue::value("IV_HEX"),
                    detail: "Initialization vector (hex).",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-padding",
                    value: OptionValue::value("PADDING"),
                    detail: "Padding mode (pkcs, oaep, none).",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
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
