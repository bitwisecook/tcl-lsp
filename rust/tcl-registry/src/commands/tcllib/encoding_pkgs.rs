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

//! tcllib binary-encoding packages — `ascii85`, `uuencode`, and `yencode`.
//!
//! Public command surface from tcllib `modules/base64/{ascii85,uuencode,
//! yencode}.man`.  These sit alongside the already-registered `base64` codec.
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// The `-file` form of the high-level encoders reads a file.
const READS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Options for `ascii85::encode`.
const ASCII85_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-maxlen",
        value: OptionValue::value("maxlen"),
        detail: "Maximum length of each output line.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-wrapchar",
        value: OptionValue::value("wrapchar"),
        detail: "Character(s) used to wrap output lines.",
        ..OptionSpec::DEFAULT
    },
];

/// Options for the high-level `uuencode::uuencode`.
const UUENCODE_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-name",
        value: OptionValue::value("string"),
        detail: "The file name recorded in the uuencode header.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-mode",
        value: OptionValue::value("octal"),
        detail: "The octal file mode recorded in the header.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-file",
        value: OptionValue::value("filename"),
        detail: "Encode the contents of a file instead of a string.",
        ..OptionSpec::DEFAULT
    },
];

/// Options for the high-level `yencode::yencode`.
const YENCODE_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-name",
        value: OptionValue::value("string"),
        detail: "The file name recorded in the yEnc header.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-line",
        value: OptionValue::value("integer"),
        detail: "Maximum encoded line length.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-crc32",
        value: OptionValue::boolean(),
        detail: "Whether to append a CRC-32 trailer.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-file",
        value: OptionValue::value("filename"),
        detail: "Encode the contents of a file instead of a string.",
        ..OptionSpec::DEFAULT
    },
];

/// A pure single-argument codec (`::pkg::encode` / `::pkg::decode`).
fn codec(
    name: &'static str,
    pkg: &'static str,
    options: &'static [OptionSpec],
    synopsis: &'static [&'static str],
    summary: &'static str,
    return_value: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::PURE,
        arity: Arity::at_least(1),
        options,
        hover: Some(HoverSnippet {
            summary,
            synopsis,
            snippet: "",
            source: "tcllib base64 module",
            examples: "",
            return_value,
        }),
        tcllib_package: Some(pkg),
        required_package: Some(pkg),
        ..CommandSpec::DEFAULT
    }
}

/// A high-level file-or-string encoder/decoder (reads a file with `-file`).
fn file_codec(
    name: &'static str,
    pkg: &'static str,
    options: &'static [OptionSpec],
    synopsis: &'static [&'static str],
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::empty(),
        arity: Arity::at_least(1),
        options,
        hover: Some(HoverSnippet {
            summary,
            synopsis,
            snippet: "",
            source: "tcllib base64 module",
            examples: "",
            return_value: "The encoded / decoded data.",
        }),
        side_effects: READS,
        tcllib_package: Some(pkg),
        required_package: Some(pkg),
        ..CommandSpec::DEFAULT
    }
}

/// All `ascii85` / `uuencode` / `yencode` command specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        codec(
            "ascii85::encode",
            "ascii85",
            ASCII85_OPTS,
            &["ascii85::encode ?-maxlen maxlen? ?-wrapchar wrapchar? string"],
            "Encode binary data using the ascii85 encoding.",
            "The ascii85-encoded representation of the input.",
        ),
        codec(
            "ascii85::decode",
            "ascii85",
            &[],
            &["ascii85::decode string"],
            "Decode an ascii85-encoded string.",
            "The decoded binary string.",
        ),
        codec(
            "uuencode::encode",
            "uuencode",
            &[],
            &["uuencode::encode string"],
            "Encode a string into raw uuencoded data (no header).",
            "The raw uuencoded data.",
        ),
        codec(
            "uuencode::decode",
            "uuencode",
            &[],
            &["uuencode::decode string"],
            "Decode raw uuencoded data (no header).",
            "The decoded binary string.",
        ),
        file_codec(
            "uuencode::uuencode",
            "uuencode",
            UUENCODE_OPTS,
            &["uuencode::uuencode ?-name string? ?-mode octal? ?-file filename | ?--? string?"],
            "Uuencode a file or string into a full uuencoded document.",
        ),
        file_codec(
            "uuencode::uudecode",
            "uuencode",
            UUDECODE_OPTS,
            &["uuencode::uudecode ?-file filename | ?--? string?"],
            "Decode a uuencoded document into its component files.",
        ),
        codec(
            "yencode::encode",
            "yencode",
            &[],
            &["yencode::encode string"],
            "Encode a string into raw yEnc data (no header).",
            "The raw yEnc-encoded data.",
        ),
        codec(
            "yencode::decode",
            "yencode",
            &[],
            &["yencode::decode string"],
            "Decode raw yEnc data (no header).",
            "The decoded binary string.",
        ),
        file_codec(
            "yencode::yencode",
            "yencode",
            YENCODE_OPTS,
            &[
                "yencode::yencode ?-name string? ?-line integer? ?-crc32 boolean? ?-file filename | ?--? string?",
            ],
            "yEnc-encode a file or string into a full yEnc document.",
        ),
        file_codec(
            "yencode::ydecode",
            "yencode",
            UUDECODE_OPTS,
            &["yencode::ydecode ?-file filename | ?--? string?"],
            "Decode a yEnc document into its component files.",
        ),
    ]
}

/// Options shared by the `-file`-only decoders (`uudecode`, `ydecode`).
const UUDECODE_OPTS: &[OptionSpec] = &[OptionSpec {
    name: "-file",
    value: OptionValue::value("filename"),
    detail: "Decode the contents of a file instead of a string.",
    ..OptionSpec::DEFAULT
}];
