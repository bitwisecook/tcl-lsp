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

//! tcllib `ripemd128` / `ripemd160` packages — the RIPEMD message digests.
//!
//! Public command surface from tcllib `modules/ripemd/ripemd128.man` and
//! `ripemd160.man`.  The one-shot `ripemd128` / `hmac128` (and the `160`
//! variants) plus the incremental token API.  The two digests are separate
//! `package require` names (`ripemd128`, `ripemd160`) sharing the `ripemd`
//! namespace; each command declares its own `required_package`.  Requires
//! Tcl 8.5+.

use crate::prelude::*;

/// `-file` / `-channel` inputs read external data.
const READS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

/// The one-shot digest / HMAC command (`ripemd128`, `hmac128`, …).
fn digest(
    name: &'static str,
    pkg: &'static str,
    keyed: bool,
    synopsis: &'static [&'static str],
    summary: &'static str,
) -> CommandSpec {
    const DIGEST_OPTS: &[OptionSpec] = &[
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
    CommandSpec {
        name,
        traits: Traits::PURE,
        arity: if keyed {
            Arity::at_least(2)
        } else {
            Arity::at_least(1)
        },
        options: if keyed { HMAC_OPTS } else { DIGEST_OPTS },
        hover: Some(HoverSnippet::brief(
            summary,
            synopsis,
            "tcllib ripemd package",
        )),
        side_effects: READS,
        tcllib_package: Some(pkg),
        required_package: Some(pkg),
        ..CommandSpec::DEFAULT
    }
}

/// An incremental token-API command.
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
        hover: Some(HoverSnippet::brief(
            summary,
            synopsis,
            "tcllib ripemd package",
        )),
        tcllib_package: Some(pkg),
        required_package: Some(pkg),
        ..CommandSpec::DEFAULT
    }
}

/// The `ripemd128` package's eight commands.
fn ripemd128_specs() -> Vec<CommandSpec> {
    vec![
        digest(
            "ripemd::ripemd128",
            "ripemd128",
            false,
            &["ripemd::ripemd128 ?-hex? ?-channel channel | -file filename? ?--? ?string?"],
            "Compute the RIPEMD-128 message-digest of a string, file, or channel.",
        ),
        digest(
            "ripemd::hmac128",
            "ripemd128",
            true,
            &["ripemd::hmac128 ?-hex? -key key ?-channel channel | -file filename? ?--? ?string?"],
            "Compute a RIPEMD-128 keyed hash-based message authentication code.",
        ),
        token_cmd(
            "ripemd::RIPEMD128Init",
            "ripemd128",
            &["ripemd::RIPEMD128Init"],
            Arity::exact(0),
            "Begin an incremental RIPEMD-128 digest, returning a state token.",
        ),
        token_cmd(
            "ripemd::RIPEMD128Update",
            "ripemd128",
            &["ripemd::RIPEMD128Update token data"],
            Arity::exact(2),
            "Add data to an in-progress RIPEMD-128 digest.",
        ),
        token_cmd(
            "ripemd::RIPEMD128Final",
            "ripemd128",
            &["ripemd::RIPEMD128Final token"],
            Arity::exact(1),
            "Finalise an incremental RIPEMD-128 digest.",
        ),
        token_cmd(
            "ripemd::RIPEHMAC128Init",
            "ripemd128",
            &["ripemd::RIPEHMAC128Init key"],
            Arity::exact(1),
            "Begin an incremental HMAC-RIPEMD-128 keyed with the given key.",
        ),
        token_cmd(
            "ripemd::RIPEHMAC128Update",
            "ripemd128",
            &["ripemd::RIPEHMAC128Update token data"],
            Arity::exact(2),
            "Add data to an in-progress HMAC-RIPEMD-128.",
        ),
        token_cmd(
            "ripemd::RIPEHMAC128Final",
            "ripemd128",
            &["ripemd::RIPEHMAC128Final token"],
            Arity::exact(1),
            "Finalise an incremental HMAC-RIPEMD-128.",
        ),
    ]
}

/// The `ripemd160` package's eight commands.
fn ripemd160_specs() -> Vec<CommandSpec> {
    vec![
        digest(
            "ripemd::ripemd160",
            "ripemd160",
            false,
            &["ripemd::ripemd160 ?-hex? ?-channel channel | -file filename? ?--? ?string?"],
            "Compute the RIPEMD-160 message-digest of a string, file, or channel.",
        ),
        digest(
            "ripemd::hmac160",
            "ripemd160",
            true,
            &["ripemd::hmac160 ?-hex? -key key ?-channel channel | -file filename? ?--? ?string?"],
            "Compute a RIPEMD-160 keyed hash-based message authentication code.",
        ),
        token_cmd(
            "ripemd::RIPEMD160Init",
            "ripemd160",
            &["ripemd::RIPEMD160Init"],
            Arity::exact(0),
            "Begin an incremental RIPEMD-160 digest, returning a state token.",
        ),
        token_cmd(
            "ripemd::RIPEMD160Update",
            "ripemd160",
            &["ripemd::RIPEMD160Update token data"],
            Arity::exact(2),
            "Add data to an in-progress RIPEMD-160 digest.",
        ),
        token_cmd(
            "ripemd::RIPEMD160Final",
            "ripemd160",
            &["ripemd::RIPEMD160Final token"],
            Arity::exact(1),
            "Finalise an incremental RIPEMD-160 digest.",
        ),
        token_cmd(
            "ripemd::RIPEHMAC160Init",
            "ripemd160",
            &["ripemd::RIPEHMAC160Init key"],
            Arity::exact(1),
            "Begin an incremental HMAC-RIPEMD-160 keyed with the given key.",
        ),
        token_cmd(
            "ripemd::RIPEHMAC160Update",
            "ripemd160",
            &["ripemd::RIPEHMAC160Update token data"],
            Arity::exact(2),
            "Add data to an in-progress HMAC-RIPEMD-160.",
        ),
        token_cmd(
            "ripemd::RIPEHMAC160Final",
            "ripemd160",
            &["ripemd::RIPEHMAC160Final token"],
            Arity::exact(1),
            "Finalise an incremental HMAC-RIPEMD-160.",
        ),
    ]
}

/// All `ripemd128` / `ripemd160` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = ripemd128_specs();
    specs.extend(ripemd160_specs());
    specs
}
