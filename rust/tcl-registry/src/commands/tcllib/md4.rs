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

//! tcllib `md4` package — the MD4 message-digest algorithm.
//!
//! Public command surface from tcllib `modules/md4/md4.man`: the one-shot
//! `md4::md4` / `md4::hmac` plus the incremental token API
//! (`MD4Init` / `MD4Update` / `MD4Final`, `HMACInit` / `HMACUpdate` /
//! `HMACFinal`).  Requires Tcl 8.5+.

use crate::prelude::*;

/// `-file` / `-channel` inputs read external data; the digest itself is
/// otherwise a pure function of its input.
const READS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

/// Options for the one-shot `md4::md4` digest command.
const MD4_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-hex",
        value: OptionValue::flag(),
        detail: "Return the digest as a hexadecimal string.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-channel",
        value: OptionValue::channel("channel"),
        detail: "Read the message from an open channel.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-file",
        value: OptionValue::value("filename"),
        detail: "Read the message from a named file.",
        ..OptionSpec::DEFAULT
    },
];

/// Options for the keyed `md4::hmac` command.
const HMAC_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-hex",
        value: OptionValue::flag(),
        detail: "Return the MAC as a hexadecimal string.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-key",
        value: OptionValue::value("key"),
        detail: "The secret key for the keyed MAC.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-channel",
        value: OptionValue::channel("channel"),
        detail: "Read the message from an open channel.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-file",
        value: OptionValue::value("filename"),
        detail: "Read the message from a named file.",
        ..OptionSpec::DEFAULT
    },
];

/// Shared per-command constructor for the incremental token API.
fn token_cmd(
    name: &'static str,
    synopsis: &'static [&'static str],
    arity: Arity,
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::empty(),
        arity,
        hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib md4 package")),
        tcllib_package: Some("md4"),
        required_package: Some("md4"),
        ..CommandSpec::DEFAULT
    }
}

/// `md4::md4 ?-hex? ?-channel channel | -file filename? ?--? ?string?`.
pub fn md4() -> CommandSpec {
    CommandSpec {
        name: "md4::md4",
        traits: Traits::PURE,
        arity: Arity::at_least(1),
        options: MD4_OPTS,
        hover: Some(HoverSnippet {
            summary: "Compute the MD4 message-digest of a string, file, or channel.",
            synopsis: &["md4::md4 ?-hex? ?-channel channel | -file filename? ?--? ?string?"],
            snippet: "",
            source: "tcllib md4 package",
            examples: "",
            return_value: "The 16-byte MD4 digest (binary, or hex with -hex).",
        }),
        side_effects: READS,
        tcllib_package: Some("md4"),
        required_package: Some("md4"),
        ..CommandSpec::DEFAULT
    }
}

/// `md4::hmac ?-hex? -key key ?-channel channel | -file filename? ?--? ?string?`.
pub fn hmac() -> CommandSpec {
    CommandSpec {
        name: "md4::hmac",
        traits: Traits::PURE,
        arity: Arity::at_least(2),
        options: HMAC_OPTS,
        hover: Some(HoverSnippet {
            summary: "Compute an MD4 keyed hash-based message authentication code.",
            synopsis: &[
                "md4::hmac ?-hex? -key key ?-channel channel | -file filename? ?--? ?string?",
            ],
            snippet: "",
            source: "tcllib md4 package",
            examples: "",
            return_value: "The HMAC-MD4 value (binary, or hex with -hex).",
        }),
        side_effects: READS,
        tcllib_package: Some("md4"),
        required_package: Some("md4"),
        ..CommandSpec::DEFAULT
    }
}

/// The incremental token API (`MD4Init`/`Update`/`Final`, HMAC variants).
pub fn incremental() -> Vec<CommandSpec> {
    vec![
        token_cmd(
            "md4::MD4Init",
            &["md4::MD4Init"],
            Arity::exact(0),
            "Begin an incremental MD4 digest, returning a state token.",
        ),
        token_cmd(
            "md4::MD4Update",
            &["md4::MD4Update token data"],
            Arity::exact(2),
            "Add data to an in-progress MD4 digest identified by its token.",
        ),
        token_cmd(
            "md4::MD4Final",
            &["md4::MD4Final token"],
            Arity::exact(1),
            "Finalise an incremental MD4 digest and return the result.",
        ),
        token_cmd(
            "md4::HMACInit",
            &["md4::HMACInit key"],
            Arity::exact(1),
            "Begin an incremental HMAC-MD4 keyed with the given key.",
        ),
        token_cmd(
            "md4::HMACUpdate",
            &["md4::HMACUpdate token data"],
            Arity::exact(2),
            "Add data to an in-progress HMAC-MD4 identified by its token.",
        ),
        token_cmd(
            "md4::HMACFinal",
            &["md4::HMACFinal token"],
            Arity::exact(1),
            "Finalise an incremental HMAC-MD4 and return the result.",
        ),
    ]
}
