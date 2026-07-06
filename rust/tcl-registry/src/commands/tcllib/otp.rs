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

//! tcllib `otp` package — RFC 2289 one-time passwords.
//!
//! Public command surface from tcllib `modules/otp/otp.man`: one command per
//! underlying digest (`otp-md4`, `otp-md5`, `otp-sha1`, `otp-rmd160`).
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// The shared option table for every `otp-*` command.
const OTP_OPTS: &[OptionSpec] = &[
    OptionSpec {
        name: "-hex",
        value: OptionValue::flag(),
        detail: "Return the one-time password as hexadecimal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-words",
        value: OptionValue::flag(),
        detail: "Return the one-time password as six RFC 2289 words.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-seed",
        value: OptionValue::value("seed"),
        detail: "The seed string (required).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-count",
        value: OptionValue::value("count"),
        detail: "The sequence number / iteration count (required).",
        ..OptionSpec::DEFAULT
    },
];

fn otp(
    name: &'static str,
    synopsis: &'static [&'static str],
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::PURE,
        arity: Arity::at_least(3),
        options: OTP_OPTS,
        hover: Some(HoverSnippet {
            summary,
            synopsis,
            snippet: "",
            source: "tcllib otp package",
            examples: "",
            return_value: "The one-time password (hex, or six words with -words).",
        }),
        tcllib_package: Some("otp"),
        required_package: Some("otp"),
        ..CommandSpec::DEFAULT
    }
}

/// All `otp` command specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        otp(
            "otp::otp-md4",
            &["otp::otp-md4 ?-hex? ?-words? -seed seed -count count data"],
            "Compute an RFC 2289 one-time password using the MD4 digest.",
        ),
        otp(
            "otp::otp-md5",
            &["otp::otp-md5 ?-hex? ?-words? -seed seed -count count data"],
            "Compute an RFC 2289 one-time password using the MD5 digest.",
        ),
        otp(
            "otp::otp-sha1",
            &["otp::otp-sha1 ?-hex? ?-words? -seed seed -count count data"],
            "Compute an RFC 2289 one-time password using the SHA-1 digest.",
        ),
        otp(
            "otp::otp-rmd160",
            &["otp::otp-rmd160 ?-hex? ?-words? -seed seed -count count data"],
            "Compute an RFC 2289 one-time password using the RIPEMD-160 digest.",
        ),
    ]
}
