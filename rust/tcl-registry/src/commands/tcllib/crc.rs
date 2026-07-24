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

//! tcllib CRC packages — `crc16`, `crc32`, `cksum`, and `sum`.
//!
//! Public command surface from tcllib `modules/crc/{crc16,crc32,cksum,sum}.man`.
//! The four `package require` names all populate the `::crc` namespace; each
//! command declares the package that provides it.  Requires Tcl 8.5+.

use crate::prelude::*;

/// `-channel` / `-filename` inputs read external data.
const READS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Options shared by every `crc16`-family checksum command.
const CRC16_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "A printf-style format for the result (e.g. 0x%04X).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-seed",
        value: OptionValue::value("value"),
        detail: "Override the initial CRC register value.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-implementation",
        value: OptionValue::command_prefix("procname"),
        detail: "Use an alternate procedure to compute the CRC.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-filename",
        value: OptionValue::value("file"),
        detail: "Compute the checksum over the contents of a file.",
        ..OptionSpec::DEFAULT
    },
];

/// Options for the `crc32` package command.
const CRC32_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "A printf-style format for the result (e.g. 0x%08X).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-seed",
        value: OptionValue::value("value"),
        detail: "Override the initial CRC register value.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-channel",
        value: OptionValue::channel("chan"),
        detail: "Compute the checksum over the contents of a channel.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-filename",
        value: OptionValue::value("file"),
        detail: "Compute the checksum over the contents of a file.",
        ..OptionSpec::DEFAULT
    },
];

/// Options for the `cksum` package command.
const CKSUM_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "A printf-style format for the result.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-chunksize",
        value: OptionValue::value("size"),
        detail: "Read the input in chunks of this many bytes.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-channel",
        value: OptionValue::channel("chan"),
        detail: "Compute the checksum over the contents of a channel.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-filename",
        value: OptionValue::value("file"),
        detail: "Compute the checksum over the contents of a file.",
        ..OptionSpec::DEFAULT
    },
];

/// Options for the `sum` package command.
const SUM_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "A printf-style format for the result.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-bsd",
        value: OptionValue::flag(),
        detail: "Use the BSD sum algorithm (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-sysv",
        value: OptionValue::flag(),
        detail: "Use the System V sum algorithm.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-chunksize",
        value: OptionValue::value("size"),
        detail: "Read the input in chunks of this many bytes.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-channel",
        value: OptionValue::channel("chan"),
        detail: "Compute the checksum over the contents of a channel.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-filename",
        value: OptionValue::value("file"),
        detail: "Compute the checksum over the contents of a file.",
        ..OptionSpec::DEFAULT
    },
];

/// The 18 named 16-bit CRC variants exposed by the `crc16` package, each with
/// its own polynomial / seed but the same calling convention.
const CRC16_VARIANTS: &[&str] = &[
    "crc16",
    "crc-ccitt",
    "xmodem",
    "kermit",
    "modbus",
    "mcrf4xx",
    "genibus",
    "crc-x25",
    "crc-sdlc",
    "crc-usb",
    "buypass",
    "umts",
    "gsm",
    "unknown2",
    "maxim",
    "unknown3",
    "unknown4",
    "cms",
];

/// One `crc16`-family checksum command.
fn crc16_cmd(variant: &'static str, synopsis: &'static [&'static str]) -> CommandSpec {
    CommandSpec {
        name: match variant {
            "crc16" => "crc::crc16",
            "crc-ccitt" => "crc::crc-ccitt",
            "xmodem" => "crc::xmodem",
            "kermit" => "crc::kermit",
            "modbus" => "crc::modbus",
            "mcrf4xx" => "crc::mcrf4xx",
            "genibus" => "crc::genibus",
            "crc-x25" => "crc::crc-x25",
            "crc-sdlc" => "crc::crc-sdlc",
            "crc-usb" => "crc::crc-usb",
            "buypass" => "crc::buypass",
            "umts" => "crc::umts",
            "gsm" => "crc::gsm",
            "unknown2" => "crc::unknown2",
            "maxim" => "crc::maxim",
            "unknown3" => "crc::unknown3",
            "unknown4" => "crc::unknown4",
            "cms" => "crc::cms",
            _ => unreachable!(),
        },
        traits: Traits::PURE,
        arity: Arity::at_least(1),
        options: CRC16_OPTS,
        hover: Some(HoverSnippet {
            summary: "Compute a 16-bit cyclic redundancy check over a message or file.",
            synopsis,
            snippet: "",
            source: "tcllib crc16 package",
            examples: "",
            return_value: "The 16-bit CRC value (formatted per -format).",
        }),
        side_effects: READS,
        tcllib_package: Some("crc16"),
        required_package: Some("crc16"),
        ..CommandSpec::DEFAULT
    }
}

/// Static per-variant synopsis strings (one owned slice per variant).
fn crc16_synopsis(variant: &str) -> &'static [&'static str] {
    match variant {
        "crc16" => &[
            "crc::crc16 ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "crc-ccitt" => &[
            "crc::crc-ccitt ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "xmodem" => &[
            "crc::xmodem ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "kermit" => &[
            "crc::kermit ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "modbus" => &[
            "crc::modbus ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "mcrf4xx" => &[
            "crc::mcrf4xx ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "genibus" => &[
            "crc::genibus ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "crc-x25" => &[
            "crc::crc-x25 ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "crc-sdlc" => &[
            "crc::crc-sdlc ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "crc-usb" => &[
            "crc::crc-usb ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "buypass" => &[
            "crc::buypass ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "umts" => &[
            "crc::umts ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "gsm" => &[
            "crc::gsm ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "unknown2" => &[
            "crc::unknown2 ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "maxim" => &[
            "crc::maxim ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "unknown3" => &[
            "crc::unknown3 ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "unknown4" => &[
            "crc::unknown4 ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        "cms" => &[
            "crc::cms ?-format format? ?-seed value? ?-implementation proc? ?-- message | -filename file?",
        ],
        _ => &["crc::crc16 ?options? message | -filename file"],
    }
}

/// An incremental token-API command (`Crc32Init`/`Update`/`Final`, etc.).
fn token_cmd(
    name: &'static str,
    pkg: &'static str,
    synopsis: &'static [&'static str],
    arity: Arity,
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::empty(),
        arity,
        hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib crc package")),
        tcllib_package: Some(pkg),
        required_package: Some(pkg),
        ..CommandSpec::DEFAULT
    }
}

/// The `crc16` package (18 named 16-bit CRC variants).
fn crc16_family() -> Vec<CommandSpec> {
    CRC16_VARIANTS
        .iter()
        .map(|v| crc16_cmd(v, crc16_synopsis(v)))
        .collect()
}

/// The `crc32` package — one-shot command plus the incremental token API.
fn crc32_family() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "crc::crc32",
            traits: Traits::PURE,
            arity: Arity::at_least(1),
            options: CRC32_OPTS,
            hover: Some(HoverSnippet {
                summary: "Compute a 32-bit cyclic redundancy check over a message, file, or channel.",
                synopsis: &[
                    "crc::crc32 ?-format format? ?-seed value? ?-channel chan | -filename file | message?",
                ],
                snippet: "",
                source: "tcllib crc32 package",
                examples: "",
                return_value: "The 32-bit CRC value (formatted per -format).",
            }),
            side_effects: READS,
            tcllib_package: Some("crc32"),
            required_package: Some("crc32"),
            ..CommandSpec::DEFAULT
        },
        token_cmd(
            "crc::Crc32Init",
            "crc32",
            &["crc::Crc32Init ?seed?"],
            Arity::new(0, 1),
            "Begin an incremental CRC-32, returning a state token.",
        ),
        token_cmd(
            "crc::Crc32Update",
            "crc32",
            &["crc::Crc32Update token data"],
            Arity::exact(2),
            "Add data to an in-progress CRC-32.",
        ),
        token_cmd(
            "crc::Crc32Final",
            "crc32",
            &["crc::Crc32Final token"],
            Arity::exact(1),
            "Finalise an incremental CRC-32 and return the result.",
        ),
    ]
}

/// The `cksum` package — one-shot command plus the incremental token API.
fn cksum_family() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "crc::cksum",
            traits: Traits::PURE,
            arity: Arity::at_least(1),
            options: CKSUM_OPTS,
            hover: Some(HoverSnippet {
                summary: "Compute a cksum(1)-compatible checksum over a string, file, or channel.",
                synopsis: &[
                    "crc::cksum ?-format format? ?-chunksize size? ?-channel chan | -filename file | string?",
                ],
                snippet: "",
                source: "tcllib cksum package",
                examples: "",
                return_value: "The cksum(1)-compatible checksum value.",
            }),
            side_effects: READS,
            tcllib_package: Some("cksum"),
            required_package: Some("cksum"),
            ..CommandSpec::DEFAULT
        },
        token_cmd(
            "crc::CksumInit",
            "cksum",
            &["crc::CksumInit"],
            Arity::exact(0),
            "Begin an incremental cksum, returning a state token.",
        ),
        token_cmd(
            "crc::CksumUpdate",
            "cksum",
            &["crc::CksumUpdate token data"],
            Arity::exact(2),
            "Add data to an in-progress cksum.",
        ),
        token_cmd(
            "crc::CksumFinal",
            "cksum",
            &["crc::CksumFinal token"],
            Arity::exact(1),
            "Finalise an incremental cksum and return the result.",
        ),
    ]
}

/// The `sum` package — a single `crc::sum` command.
fn sum_cmd() -> CommandSpec {
    CommandSpec {
        name: "crc::sum",
        traits: Traits::PURE,
        arity: Arity::at_least(1),
        options: SUM_OPTS,
        hover: Some(HoverSnippet {
            summary: "Compute a sum(1)-compatible checksum over a string, file, or channel.",
            synopsis: &[
                "crc::sum ?-bsd | -sysv? ?-format format? ?-chunksize size? ?-channel chan | -filename file | string?",
            ],
            snippet: "",
            source: "tcllib sum package",
            examples: "",
            return_value: "The sum(1)-compatible checksum value.",
        }),
        side_effects: READS,
        tcllib_package: Some("sum"),
        required_package: Some("sum"),
        ..CommandSpec::DEFAULT
    }
}

/// All CRC / checksum command specs across the four packages.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = crc16_family();
    specs.extend(crc32_family());
    specs.extend(cksum_family());
    specs.push(sum_cmd());
    specs
}
