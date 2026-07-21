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

//! `unicode` — Unicode normalization (Tcl 9.1).
//!
//! `unicode to<form> ?-profile PROFILE? STRING` converts a string to one of
//! the four Unicode normalization forms (NFC / NFD / NFKC / NFKD).  The
//! optional `-profile` selects the encoding profile used for invalid input.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unicode subcommand ?-profile PROFILE? STRING",
    dialects: None,
}];

static PROFILE_OPTIONS: [OptionSpec; 1] = [OptionSpec {
    name: "-profile",
    value: OptionValue::value("PROFILE"),
    detail: "Encoding profile used for invalid input.",
    dialects: None,
    aliases: &[],
    min_version: None,
}];

/// One `unicode to<form>` normalization subcommand: `?-profile PROFILE? STRING`
/// (1 positional + optional value-bearing flag ⇒ 1–3 words).
const fn normalise_sub(
    name: &'static str,
    synopsis: &'static str,
    detail: &'static str,
) -> SubCommand {
    SubCommand {
        name,
        arity: Arity::new(1, 3),
        detail,
        synopsis,
        options: &PROFILE_OPTIONS,
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 4] = [
    normalise_sub(
        "tonfc",
        "unicode tonfc ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form C (NFC).",
    ),
    normalise_sub(
        "tonfd",
        "unicode tonfd ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form D (NFD).",
    ),
    normalise_sub(
        "tonfkc",
        "unicode tonfkc ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form KC (NFKC).",
    ),
    normalise_sub(
        "tonfkd",
        "unicode tonfkd ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form KD (NFKD).",
    ),
];

/// Command spec for `unicode` (Tcl 9.1).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unicode",
        dialects: Some(DialectSet::TCL91),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        hover: Some(HoverSnippet::brief(
            "Unicode normalization (Tcl 9.1).",
            &["unicode subcommand ?-profile PROFILE? STRING"],
            "Tcl man page unicode.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
