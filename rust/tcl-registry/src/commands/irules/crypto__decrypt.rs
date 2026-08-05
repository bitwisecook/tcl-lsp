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

//! `CRYPTO::decrypt` iRules command.
use crate::prelude::*;
/// The command's option table, hoisted out of the spec literal so the
/// builder stays inside the line budget.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-alg",
        value: OptionValue::value("ALG"),
        detail: "Decryption algorithm.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-ctx",
        value: OptionValue::value("CTX_VAR"),
        detail: "Context variable for multi-step operations.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-final",
        value: OptionValue::flag(),
        detail: "Finalize context-based operation.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-key",
        value: OptionValue::value("KEY"),
        detail: "Binary key.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-keyhex",
        value: OptionValue::value("KEY_HEX"),
        detail: "Hex-encoded key.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-iv",
        value: OptionValue::value("IV"),
        detail: "Initialization vector (binary).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-ivhex",
        value: OptionValue::value("IV_HEX"),
        detail: "Initialization vector (hex).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("PADDING"),
        detail: "Padding mode (pkcs, oaep, none).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::decrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This iRules command decrypts data.",
            synopsis: &["CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )"],
            snippet: "This iRules command decrypts data.\n\nCRYPTO::decrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n                [-padding <\"pkcs\" | \"oaep\" | \"none\">]\n\n     * decrypts data based on several parameters\n          + alg - algorithm. ASCII string from a given list (see below)\n            The spelling is lowercase and the iRule will fail for anything\n            not in the list. In ctx mode, alg must be given in the first\n            CRYPTO::command and cannot be modified.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__decrypt.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
            dialects: None,
        }],
        options: OPTIONS,
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
