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

//! `unicode` — Unicode string normalization (Tcl 9.1).
//!
//! New in Tcl 9.1 (TIP 726 — the `.VS "TIP726"` version-stamp macros
//! bracket each of the four subcommand entries in `doc/unicode.n`'s nroff
//! source): `unicode.n` exists only in the 9.1 `TclCmd` manpage tree —
//! the 8.4, 8.5, 8.6 (`.htm`, not `.html` — the 8.6 tree redirects
//! `.html` to a broken `Location`), and 9.0 trees were each fetched
//! individually and confirmed to 404, rather than assumed absent from the
//! 9.0/9.1 gap alone. Real Tcl 9.1b0 source
//! (`generic/tclBasic.c`'s `ensembleCommands` table) registers it as an
//! ensemble (`tclUnicodeImplMap`) flagged `CMD_IS_SAFE` — unlike
//! `encoding`/`clock`/`file` (each flag `0` in that same table),
//! `unicode` is *not* hidden from a safe interpreter, so
//! `Traits::SAFE_INTERP_HIDDEN` does not apply here.
//!
//! `unicode function ?options? string` (the literal SYNOPSIS wording;
//! `function` is one of `tonfc`/`tonfd`/`tonfkc`/`tonfkd`) converts
//! `string` to one of the four Unicode normalization forms defined in
//! Section 3.11 of the Unicode standard. Each form additionally takes an
//! optional `-profile profile` (`strict`, the default, or `replace` —
//! unlike `encoding convertfrom`/`convertto`'s `-profile`, `unicode` does
//! not accept the legacy `tcl8` member) controlling how invalid/ill-formed
//! Unicode data in `string` is handled; see the `encoding` command's
//! PROFILES section for the semantics shared by every profile-aware
//! command.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unicode function ?options? string",
    dialects: None,
}];

/// `-profile`'s values (per `unicode.n`, which for details defers to
/// `encoding(n)`'s PROFILES section) — `unicode` accepts only these two;
/// unlike `encoding convertfrom`/`convertto` it does *not* accept the
/// legacy `tcl8` profile (there is no Tcl 8.6 normalization behaviour to
/// reproduce). `strict` is the default when `-profile` is not given, per
/// the general PROFILES-section rule `unicode.n` explicitly defers to.
const PROFILE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "strict",
        detail: "Default. Raises an exception immediately on invalid or ill-formed Unicode data.",
        min_tcl: Some(TclVersion::V9_1),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "replace",
        detail: "Continues processing, substituting a Unicode-conformant replacement for invalid or ill-formed Unicode data instead of raising an exception.",
        min_tcl: Some(TclVersion::V9_1),
        ..ArgValue::DEFAULT
    },
];

static PROFILE_OPTIONS: [OptionSpec; 1] = [OptionSpec {
    name: "-profile",
    value: OptionValue::Takes(OptionArg {
        values: PROFILE_VALUES,
        closed: true,
        hint: "profile",
        ..OptionArg::DEFAULT
    }),
    detail: "Profile controlling how the command reacts to invalid or ill-formed Unicode data in string; must be strict or replace, defaulting to strict when omitted.",
    dialects: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
}];

/// One `unicode to<form>` normalization subcommand: `?-profile profile? string`
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
        "unicode tonfc ?-profile profile? string",
        "Return string normalized to Unicode Normalization Form C (NFC): canonical decomposition followed by canonical composition.",
    ),
    normalise_sub(
        "tonfd",
        "unicode tonfd ?-profile profile? string",
        "Return string normalized to Unicode Normalization Form D (NFD): canonical decomposition.",
    ),
    normalise_sub(
        "tonfkc",
        "unicode tonfkc ?-profile profile? string",
        "Return string normalized to Unicode Normalization Form KC (NFKC): compatibility decomposition followed by canonical composition.",
    ),
    normalise_sub(
        "tonfkd",
        "unicode tonfkd ?-profile profile? string",
        "Return string normalized to Unicode Normalization Form KD (NFKD): compatibility decomposition.",
    ),
];

/// Command spec for `unicode` (Tcl 9.1 only — confirmed absent from the
/// 8.4, 8.5, 8.6, and 9.0 `TclCmd` manpage trees, each fetched and
/// checked individually rather than assumed from a neighbouring version).
///
/// `Traits::CSE_CANDIDATE` alongside `BYTE_COMPILED`: every subcommand is
/// an unconditionally pure, deterministic string transform (`pure: true`
/// on all four, no `mutator`), the same shape `string`/`dict` — both
/// `CSE_CANDIDATE` — use for their pure subcommands; unlike `encoding`/
/// `timer`, `unicode` has no impure/scheduling subcommand dragging the
/// ensemble's average value down, so the whole command earns the trait.
/// No `Traits::PURE`, matching every other ensemble in this codebase
/// (`string`/`dict`/`array`/`encoding`/`timer`): purity is expressed per
/// subcommand (`pure: true` above) and `tcl_compiler::side_effects`
/// deliberately reads that instead of a command-level blanket flag for
/// ensembles.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unicode",
        traits: Traits::BYTE_COMPILED | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::TCL91),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        hover: Some(HoverSnippet {
            summary: "Normalize a string to one of the four Unicode normalization forms (Tcl 9.1).",
            synopsis: &[
                "unicode tonfc ?-profile profile? string",
                "unicode tonfd ?-profile profile? string",
                "unicode tonfkc ?-profile profile? string",
                "unicode tonfkd ?-profile profile? string",
            ],
            snippet: "New in Tcl 9.1. function selects one of four Unicode normalization forms, applied to string: tonfd returns the fully canonically-decomposed form, and tonfc additionally recomposes it (canonical decomposition followed by canonical composition); tonfkd returns the fully compatibility-decomposed form, and tonfkc additionally recomposes it (compatibility decomposition followed by canonical composition). Compatibility decomposition is the more aggressive of the two — it can collapse formatting distinctions (ligatures, circled/superscript/width variants, and similar) that canonical decomposition leaves alone. The four forms are defined in Section 3.11 of the Unicode standard.\n\nEvery form takes the same optional -profile profile, controlling how the command reacts to invalid or ill-formed Unicode data in string: strict (the default when -profile is omitted) raises an exception; replace continues processing, substituting a Unicode-conformant replacement instead. See the PROFILES section of the encoding command's manpage for the semantics shared by every profile-aware command — unlike encoding convertfrom/convertto, unicode does not accept the legacy tcl8 profile.",
            source: "Tcl unicode(n)",
            examples: "set s \"e\\u0301\"\nputs [string length $s]              ;# 2: e + combining acute accent\nset composed [unicode tonfc $s]\nputs [string length $composed]       ;# 1: precomposed U+00E9\nset decomposed [unicode tonfd $composed]\nputs [string equal $decomposed $s]   ;# 1: round-trips to the decomposed form\n\n# Compatibility folding: full-width Latin letters collapse to plain ASCII\nputs [unicode tonfkc \\uFF21\\uFF22\\uFF23]   ;# ABC\n\nif {[catch {unicode tonfc -profile strict $data} result]} {\n    puts \"invalid Unicode data: $result\"\n}",
            return_value: "The normalized copy of string. With the default -profile strict, invalid or ill-formed Unicode data in string raises an exception rather than returning a value; with -profile replace, the command substitutes a Unicode-conformant replacement and returns the normalized result instead.",
        }),
        ..CommandSpec::DEFAULT
    }
}
