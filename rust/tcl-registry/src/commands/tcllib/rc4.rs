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

//! tcllib `rc4` package — the RC4 stream cipher.
//!
//! Public command surface from tcllib `modules/rc4/rc4.man`: the one-shot
//! `rc4::rc4` plus the incremental `RC4Init` / `RC4` / `RC4Final` token API.
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// `-in` / `-out` channels perform I/O on the one-shot command.
const IO: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Options for the one-shot `rc4::rc4` command.
const RC4_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-hex",
        value: OptionValue::flag(),
        detail: "Return the result as a hexadecimal string.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-key",
        value: OptionValue::value("keyvalue"),
        detail: "The RC4 key (required).",
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
];

fn token(
    name: &'static str,
    synopsis: &'static [&'static str],
    arity: Arity,
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::empty(),
        arity,
        hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib rc4 package")),
        tcllib_package: Some("rc4"),
        required_package: Some("rc4"),
        ..CommandSpec::DEFAULT
    }
}

/// All `rc4` command specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "rc4::rc4",
            traits: Traits::empty(),
            arity: Arity::at_least(2),
            options: RC4_OPTS,
            hover: Some(HoverSnippet {
                summary: "Encrypt or decrypt data with the RC4 stream cipher.",
                synopsis: &["rc4::rc4 ?-hex? -key keyvalue ?-in channel? ?-out channel? ?--? data"],
                snippet: "",
                source: "tcllib rc4 package",
                examples: "",
                return_value: "The transformed data (binary, or hex with -hex).",
            }),
            side_effects: IO,
            tcllib_package: Some("rc4"),
            required_package: Some("rc4"),
            ..CommandSpec::DEFAULT
        },
        token(
            "rc4::RC4Init",
            &["rc4::RC4Init keydata"],
            Arity::exact(1),
            "Initialise an RC4 key schedule and return a state token.",
        ),
        token(
            "rc4::RC4",
            &["rc4::RC4 Key data"],
            Arity::exact(2),
            "Transform data through an in-progress RC4 keystream.",
        ),
        token(
            "rc4::RC4Final",
            &["rc4::RC4Final Key"],
            Arity::exact(1),
            "Release the resources held by an RC4 state token.",
        ),
    ]
}
