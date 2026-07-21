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

//! tcllib block-cipher packages — `aes`, `blowfish`, and `des` (triple-DES).
//!
//! Each shares the same public API from tcllib
//! `modules/{aes,blowfish,des}/*.man`: a one-shot high-level command
//! (`aes::aes`, `blowfish::blowfish`, `DES::des`) plus the incremental
//! programming interface (`Init` / `Encrypt` / `Decrypt` / `Reset` / `Final`).
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// `-in` / `-out` channels perform I/O.
const IO: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

/// Direction values, shared by every block cipher.
const DIR_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "encrypt",
        detail: "Encrypt the input (the default).",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "decrypt",
        detail: "Decrypt the input.",
        min_tcl: None,
        code: None,
    },
];

/// `aes` cipher modes.
const AES_MODES: &[ArgValue] = &[
    ArgValue {
        value: "ecb",
        detail: "Electronic codebook mode.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "cbc",
        detail: "Cipher block chaining mode (the default).",
        min_tcl: None,
        code: None,
    },
];

/// `des` / triple-DES cipher modes.
const DES_MODES: &[ArgValue] = &[
    ArgValue {
        value: "ecb",
        detail: "Electronic codebook mode.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "cbc",
        detail: "Cipher block chaining mode (the default).",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "cfb",
        detail: "Cipher feedback mode.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "ofb",
        detail: "Output feedback mode.",
        min_tcl: None,
        code: None,
    },
];

/// The static option table for one block cipher's high-level command.
struct CipherOpts {
    primary: &'static [OptionSpec],
}

const AES_OPTS: CipherOpts = CipherOpts {
    primary: &[
        OptionSpec {
            name: "-mode",
            value: OptionValue::enumerated(AES_MODES, true, "mode"),
            detail: "Cipher mode of operation (default cbc).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-dir",
            value: OptionValue::enumerated(DIR_VALUES, true, "direction"),
            detail: "Whether to encrypt or decrypt (default encrypt).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-key",
            value: OptionValue::value("keydata"),
            detail: "The 16-, 24-, or 32-byte binary key (required).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-iv",
            value: OptionValue::value("vector"),
            detail: "The 16-byte initialisation vector for CBC mode.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-hex",
            value: OptionValue::flag(),
            detail: "Return the result as a hexadecimal string.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-in",
            value: OptionValue::channel("channel"),
            detail: "Read the input data from a channel.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-out",
            value: OptionValue::channel("channel"),
            detail: "Write the result to a channel.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-chunksize",
            value: OptionValue::value("size"),
            detail: "Process channel input in chunks of this size.",
            ..OptionSpec::DEFAULT
        },
    ],
};

const BLOWFISH_OPTS: CipherOpts = CipherOpts {
    primary: &[
        OptionSpec {
            name: "-mode",
            value: OptionValue::enumerated(AES_MODES, true, "mode"),
            detail: "Cipher mode of operation (default cbc).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-dir",
            value: OptionValue::enumerated(DIR_VALUES, true, "direction"),
            detail: "Whether to encrypt or decrypt (default encrypt).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-key",
            value: OptionValue::value("keydata"),
            detail: "The variable-length binary key (required).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-iv",
            value: OptionValue::value("vector"),
            detail: "The 8-byte initialisation vector for CBC mode.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-hex",
            value: OptionValue::flag(),
            detail: "Return the result as a hexadecimal string.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-in",
            value: OptionValue::channel("channel"),
            detail: "Read the input data from a channel.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-out",
            value: OptionValue::channel("channel"),
            detail: "Write the result to a channel.",
            ..OptionSpec::DEFAULT
        },
    ],
};

const DES_OPTS: CipherOpts = CipherOpts {
    primary: &[
        OptionSpec {
            name: "-mode",
            value: OptionValue::enumerated(DES_MODES, true, "mode"),
            detail: "Cipher mode of operation (default cbc).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-dir",
            value: OptionValue::enumerated(DIR_VALUES, true, "direction"),
            detail: "Whether to encrypt or decrypt (default encrypt).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-key",
            value: OptionValue::value("keydata"),
            detail: "The 8-byte (DES) or 24-byte (triple-DES) key (required).",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-iv",
            value: OptionValue::value("vector"),
            detail: "The 8-byte initialisation vector for chaining modes.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-weak",
            value: OptionValue::flag(),
            detail: "Permit known weak keys instead of rejecting them.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-hex",
            value: OptionValue::flag(),
            detail: "Return the result as a hexadecimal string.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-in",
            value: OptionValue::channel("channel"),
            detail: "Read the input data from a channel.",
            ..OptionSpec::DEFAULT
        },
        OptionSpec {
            name: "-out",
            value: OptionValue::channel("channel"),
            detail: "Write the result to a channel.",
            ..OptionSpec::DEFAULT
        },
    ],
};

/// Build one block cipher's five-command surface.
///
/// `ns` is the command namespace (`aes`, `blowfish`, `DES`); `pkg` is the
/// `package require` name.  `primary` names the high-level command word and
/// carries the full documented option table.
struct Cipher {
    ns: &'static str,
    pkg: &'static str,
    primary_name: &'static str,
    primary_synopsis: &'static [&'static str],
    primary_summary: &'static str,
    opts: CipherOpts,
    init_name: &'static str,
    encrypt_name: &'static str,
    decrypt_name: &'static str,
    reset_name: &'static str,
    final_name: &'static str,
}

fn cipher_specs(c: &Cipher) -> Vec<CommandSpec> {
    let token = |name: &'static str,
                 synopsis: &'static [&'static str],
                 arity: Arity,
                 summary: &'static str|
     -> CommandSpec {
        CommandSpec {
            name,
            traits: Traits::empty(),
            arity,
            hover: Some(HoverSnippet::brief(
                summary,
                synopsis,
                "tcllib cipher package",
            )),
            tcllib_package: Some(c.pkg),
            required_package: Some(c.pkg),
            ..CommandSpec::DEFAULT
        }
    };
    vec![
        CommandSpec {
            name: c.primary_name,
            traits: Traits::empty(),
            arity: Arity::at_least(2),
            options: c.opts.primary,
            hover: Some(HoverSnippet {
                summary: c.primary_summary,
                synopsis: c.primary_synopsis,
                snippet: "",
                source: "tcllib block-cipher package",
                examples: "",
                return_value: "The transformed data (binary, or hex with -hex).",
            }),
            side_effects: IO,
            tcllib_package: Some(c.pkg),
            required_package: Some(c.pkg),
            ..CommandSpec::DEFAULT
        },
        token(
            c.init_name,
            match c.ns {
                "aes" => &["aes::Init mode keydata iv"],
                "blowfish" => &["blowfish::Init mode keydata iv"],
                _ => &["DES::Init mode keydata iv ?weak?"],
            },
            if c.ns == "DES" {
                Arity::new(3, 4)
            } else {
                Arity::exact(3)
            },
            "Construct a key schedule and return an opaque cipher-state token.",
        ),
        token(
            c.encrypt_name,
            match c.ns {
                "aes" => &["aes::Encrypt Key data"],
                "blowfish" => &["blowfish::Encrypt Key data"],
                _ => &["DES::Encrypt Key data"],
            },
            Arity::exact(2),
            "Encrypt a block of data using a cipher-state token.",
        ),
        token(
            c.decrypt_name,
            match c.ns {
                "aes" => &["aes::Decrypt Key data"],
                "blowfish" => &["blowfish::Decrypt Key data"],
                _ => &["DES::Decrypt Key data"],
            },
            Arity::exact(2),
            "Decrypt a block of data using a cipher-state token.",
        ),
        token(
            c.reset_name,
            match c.ns {
                "aes" => &["aes::Reset Key iv"],
                "blowfish" => &["blowfish::Reset Key iv"],
                _ => &["DES::Reset Key iv"],
            },
            Arity::exact(2),
            "Reset a cipher-state token's chaining state with a new IV.",
        ),
        token(
            c.final_name,
            match c.ns {
                "aes" => &["aes::Final Key"],
                "blowfish" => &["blowfish::Final Key"],
                _ => &["DES::Final Key"],
            },
            Arity::exact(1),
            "Release the resources held by a cipher-state token.",
        ),
    ]
}

/// All block-cipher command specs (`aes`, `blowfish`, `DES`).
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(cipher_specs(&Cipher {
        ns: "aes",
        pkg: "aes",
        primary_name: "aes::aes",
        primary_synopsis: &["aes::aes ?-mode [ecb|cbc]? ?-dir [encrypt|decrypt]? -key keydata ?-iv vector? ?-hex? ?-out channel? ?-chunksize size? ?-in channel | ?--? data?"],
        primary_summary: "Encrypt or decrypt data with the AES block cipher.",
        opts: AES_OPTS,
        init_name: "aes::Init",
        encrypt_name: "aes::Encrypt",
        decrypt_name: "aes::Decrypt",
        reset_name: "aes::Reset",
        final_name: "aes::Final",
    }));
    specs.extend(cipher_specs(&Cipher {
        ns: "blowfish",
        pkg: "blowfish",
        primary_name: "blowfish::blowfish",
        primary_synopsis: &["blowfish::blowfish ?-mode [ecb|cbc]? ?-dir [encrypt|decrypt]? -key keydata ?-iv vector? ?-hex? ?-out channel? ?-in channel | ?--? data?"],
        primary_summary: "Encrypt or decrypt data with the Blowfish block cipher.",
        opts: BLOWFISH_OPTS,
        init_name: "blowfish::Init",
        encrypt_name: "blowfish::Encrypt",
        decrypt_name: "blowfish::Decrypt",
        reset_name: "blowfish::Reset",
        final_name: "blowfish::Final",
    }));
    specs.extend(cipher_specs(&Cipher {
        ns: "DES",
        pkg: "des",
        primary_name: "DES::des",
        primary_synopsis: &["DES::des ?-mode [ecb|cbc|cfb|ofb]? ?-dir [encrypt|decrypt]? -key keydata ?-iv vector? ?-weak? ?-hex? ?-out channel? ?-in channel | ?--? data?"],
        primary_summary: "Encrypt or decrypt data with the DES / triple-DES block cipher.",
        opts: DES_OPTS,
        init_name: "DES::Init",
        encrypt_name: "DES::Encrypt",
        decrypt_name: "DES::Decrypt",
        reset_name: "DES::Reset",
        final_name: "DES::Final",
    }));
    specs
}
