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

//! `CRYPTO::keygen` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::keygen",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Generates keys that can be used to encrypt and sign data.",
            synopsis: &["CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))"],
            snippet: "This iRules command is used to generate keys that can be used to\nencrypt and sign data.\n\nCRYPTO::keygen -alg <> -len <> [-passphrase <> -salt[hex] <> -rounds <>]\n\n     * Used to generate keys that can be used to encrypt and sign data.\n          + -alg (Two options: random or pbkdf2-md5)\n          + -len (Must be a multiple of 8, e.g.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__keygen.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-alg",
                    value: OptionValue::value("ALG"),
                    detail: "Key generation algorithm.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-len",
                    value: OptionValue::value("LENGTH"),
                    detail: "Key length (must be multiple of 8).",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-exp",
                    value: OptionValue::value("EXPONENT"),
                    detail: "Exponent (for RSA).",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-passphrase",
                    value: OptionValue::value("PASSPHRASE"),
                    detail: "Passphrase for key derivation.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-salt",
                    value: OptionValue::value("SALT"),
                    detail: "Binary salt.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-salthex",
                    value: OptionValue::value("SALT_HEX"),
                    detail: "Hex-encoded salt.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-rounds",
                    value: OptionValue::value("ROUNDS"),
                    detail: "Rounds for PBKDF2.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
