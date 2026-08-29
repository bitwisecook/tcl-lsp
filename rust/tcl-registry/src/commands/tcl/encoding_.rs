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

//! `encoding` — manipulate character encodings.
//!
//! `convertfrom`/`convertto`/`names`/`system` date to Tcl 8.4; `dirs` was
//! added in 8.5. Tcl 9.0 (TIP 656) added the `profiles` and `user`
//! subcommands, plus a `-profile`/`-failindex` option pair on
//! `convertfrom`/`convertto` that switches the default error-handling
//! behaviour from Tcl 8.6's lenient byte-mapping to a strict,
//! Unicode-conformant default. See each `SubCommand`'s `dialects` field for
//! exact version gating.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "encoding subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// `-profile`'s values (`encoding(n)` PROFILES section) — Tcl 9.0+ only
/// (absent from every 8.4/8.5/8.6 encoding.n fetch; present, with these
/// exact three members and wording, in both 9.0.4 and 9.1b0). `strict` is
/// the default when `-profile` is not given — a behaviour change from pre-9.0
/// Tcl, which always behaved like `tcl8` (no `-profile` option existed to
/// select otherwise).
const PROFILE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "strict",
        detail: "Default. Stops processing on the first conversion error, reporting it as an exception (or, with -failindex, returning the partial result instead). Unicode-conformant.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "tcl8",
        detail: "Lenient, Tcl 8.6-compatible behaviour: convertfrom maps invalid bytes to their numerically equivalent code points; convertto replaces an unrepresentable character with an encoding-dependent fallback, usually '?'.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "replace",
        detail: "Continues processing, substituting invalid/unrepresentable data with a fallback: U+FFFD on convertfrom; on convertto, U+FFFD for UTF targets or an encoding-specific fallback (usually '?') otherwise.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
];

/// Options on `encoding convertfrom`, added in Tcl 9.0 (TIP 656) — absent
/// from the 8.4/8.5/8.6 synopsis and DESCRIPTION; present, unchanged, in
/// 9.0.4 and 9.1b0. The manual documents them only on a second synopsis
/// line, `encoding convertfrom ?-profile profile? ?-failindex var? encoding
/// data`, where — unlike the plain form — `encoding` is not optional.
const CONVERTFROM_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-profile",
        value: OptionValue::Takes(OptionArg {
            values: PROFILE_VALUES,
            closed: true,
            hint: "profile",
            ..OptionArg::DEFAULT
        }),
        detail: "Encoding profile controlling how a conversion error is handled; defaults to strict.",
        surface: Some(SpecSurface::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-failindex",
        value: OptionValue::Takes(OptionArg {
            role: ArgRole::VarWrite,
            hint: "var",
            ..OptionArg::DEFAULT
        }),
        detail: "Variable to receive the index of the source byte that triggered a conversion error, or -1 if none did. When given, returns the successfully-converted prefix instead of raising an exception on failure.",
        surface: Some(SpecSurface::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// Options on `encoding convertto` — the same Tcl 9.0 (TIP 656) addition as
/// [`CONVERTFROM_OPTIONS`], mirrored with `-failindex`'s direction-specific
/// wording: here it reports a *character* index into the source Tcl string,
/// versus a *byte* index into the source data for `convertfrom`.
const CONVERTTO_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-profile",
        value: OptionValue::Takes(OptionArg {
            values: PROFILE_VALUES,
            closed: true,
            hint: "profile",
            ..OptionArg::DEFAULT
        }),
        detail: "Encoding profile controlling how a conversion error is handled; defaults to strict.",
        surface: Some(SpecSurface::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-failindex",
        value: OptionValue::Takes(OptionArg {
            role: ArgRole::VarWrite,
            hint: "var",
            ..OptionArg::DEFAULT
        }),
        detail: "Variable to receive the index of the source character that triggered a conversion error, or -1 if none did. When given, returns the successfully-converted prefix instead of raising an exception on failure.",
        surface: Some(SpecSurface::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "convertfrom",
        traits: Traits::TAINT_SOURCE,
        // Base form is 1 ("data" alone) to 2 ("encoding data") positional
        // words; the Tcl 9.0+ `-profile`/`-failindex` pair (each a flag +
        // value, TCL90_PLUS-gated on the options below) can add up to 4
        // more words on top of the mandatory "encoding data" pair.
        arity: Arity::new(1, 6),
        detail: "Convert data from the specified encoding to a Tcl string; if encoding is omitted, uses the current system encoding.",
        synopsis: "encoding convertfrom ?encoding? data",
        options: CONVERTFROM_OPTIONS,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            // -failindex (Tcl 9.0+) optionally writes its `var` argument;
            // the static spec can't see whether a given call uses it, so
            // conservatively declare the write rather than staying `pure`.
            target: SideEffectTarget::Variable,
            writes: true,
            surface: Some(SpecSurface::TCL90_PLUS),
            ..SideEffect::DEFAULT
        }],
        is_unescape: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "convertto",
        arity: Arity::new(1, 6),
        detail: "Convert a Tcl string to the specified encoding, returning a byte sequence; if encoding is omitted, uses the current system encoding.",
        synopsis: "encoding convertto ?encoding? data",
        options: CONVERTTO_OPTIONS,
        // S110: encodes its character operand into a fresh byte array (the
        // `ByteArray` return type marks it a binary source — tclsh
        // 8.6.14-verified: `tcl::unsupported::representation [encoding
        // convertto utf-8 héllo]` → bytearray). On an *already-binary*
        // operand this is the double-encode bug: the bytes are reinterpreted
        // as latin-1 characters and re-encoded — 8.6.14-verified, byte `200`
        // → bytes `C3 88`. Identical in 9.0 (`Tcl_UtfToExternalDStringEx`);
        // TIP 568 does not change `convertto` because its operand is read as
        // a *string*, not as bytes.
        byte_array_effect: ByteArrayEffect::Encodes,
        return_type: Some(TclType::ByteArray),
        side_effects: &[SideEffect {
            // -failindex (Tcl 9.0+) optionally writes its `var` argument;
            // see the identical note on `convertfrom` above.
            target: SideEffectTarget::Variable,
            writes: true,
            surface: Some(SpecSurface::TCL90_PLUS),
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dirs",
        // A single optional list-valued argument — `encoding dirs
        // ?directoryList?` — not a vararg tail.
        arity: Arity::new(0, 1),
        detail: "Get or set the search path for *.enc encoding data files. It is an error for directoryList not to be a valid list; an unreadable/unsearchable directory in it is silently ignored when searching.",
        synopsis: "encoding dirs ?directoryList?",
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            // Getter when called with 0 args, setter (interpreter-wide
            // state) with 1 — the static spec can't see which, so declare
            // both.
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        // Added in Tcl 8.5 — absent from the 8.4 synopsis and DESCRIPTION;
        // present, unchanged, in 8.5 through 9.1.
        surface: Some(SpecSurface::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return a list of the names of all currently available encodings. utf-8 and iso8859-1 are always present.",
        synopsis: "encoding names",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        arity: Arity::new(0, 1),
        detail: "Get or set the system encoding, used whenever Tcl passes strings to system calls.",
        synopsis: "encoding system ?encoding?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            // Getter when called with 0 args, setter (interpreter-wide
            // state) with 1 — the static spec can't see which, so declare
            // both.
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "profiles",
        arity: Arity::exact(0),
        // `encoding profiles` was added in Tcl 9.0 (TIP 656) — absent from
        // the 8.4/8.5/8.6 synopsis; present, unchanged, in 9.0.4 and 9.1b0.
        surface: Some(SpecSurface::TCL90_PLUS),
        detail: "Return a list of the names of the available encoding profiles (strict, tcl8, replace).",
        synopsis: "encoding profiles",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "user",
        arity: Arity::exact(0),
        // `encoding user` was added in Tcl 9.0 (TIP 656) — absent from the
        // 8.4/8.5/8.6 synopsis; present, unchanged, in 9.0.4 and 9.1b0.
        surface: Some(SpecSurface::TCL90_PLUS),
        detail: "Return the encoding matching the user's platform preferences: on Windows, derived from the user's code-page registry setting; on other platforms, the same value as encoding system.",
        synopsis: "encoding user",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "encoding",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::BYTE_COMPILED | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Convert between character encodings, and manage encoding-related interpreter state.",
            synopsis: &[
                "encoding convertfrom ?encoding? data",
                "encoding convertfrom ?-profile profile? ?-failindex var? encoding data",
                "encoding convertto ?encoding? data",
                "encoding convertto ?-profile profile? ?-failindex var? encoding data",
                "encoding dirs ?directoryList?",
                "encoding names",
                "encoding profiles",
                "encoding system ?encoding?",
                "encoding user",
            ],
            snippet: "convertfrom/convertto move data between Tcl's internal string representation and an external encoding, defaulting to the current system encoding when encoding is omitted. names lists the available encodings (utf-8 and iso8859-1 are always present); system gets or sets the encoding used whenever Tcl passes strings to system calls. dirs (Tcl 8.5+) sets the search path for additional *.enc encoding data files. Tcl 9.0 added the profiles and user subcommands, plus a -profile/-failindex option pair on convertfrom/convertto (TIP 656): -profile picks how a conversion error is handled — strict (the default) raises an exception, tcl8 reproduces Tcl 8.6's lenient byte-mapping/'?'-substitution behaviour, and replace substitutes U+FFFD (or '?' on non-UTF convertto targets) — and -failindex var returns the successfully-converted prefix instead of raising, storing the index of the failing byte (convertfrom) or character (convertto) in var, or -1 if none failed. Code relying on Tcl 8.6's old silent lenient behaviour needs -profile tcl8 under Tcl 9.0+, since strict is now the default.",
            source: "Tcl encoding(n)",
            examples: "encoding names\nencoding system\nset s [encoding convertfrom utf-8 $bytes]\nset bytes [encoding convertto -profile strict iso8859-1 $s]\nif {[catch {encoding convertfrom -profile strict -failindex idx ascii $data} result]} {\n    puts \"bad byte at $idx: $result\"\n}",
            return_value: "Depends on the subcommand: convertfrom returns the decoded string, convertto the encoded byte sequence; dirs/names/profiles return a list; system/user return an encoding name. dirs and system also act as setters, in which case they return the empty string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
