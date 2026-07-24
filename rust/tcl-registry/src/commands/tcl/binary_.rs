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

//! `binary` — manipulate binary data.
use crate::prelude::*;

// Only `binary scan` ever reaches this: `format`/`encode`/`decode` are
// `pure: true` on their `SubCommand` entries, so the compiler's side-effect
// classifier (`classify_side_effects`) short-circuits on subcommand purity
// before it ever consults this command-level fallback — see
// `tcl-compiler/src/side_effects.rs`'s `sub.pure && !sub.mutator` branch.
// `scan` writes fresh values into its `varName` targets (it does not read
// their prior contents), matching `scan_.rs`'s identical declaration.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary format formatString ?arg arg ...?",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary scan string formatString ?varName varName ...?",
        dialects: None,
    },
    // `binary encode`/`binary decode` were added in Tcl 8.6 (TIP 317) — see
    // the matching gate on the `encode`/`decode` `SubCommand` entries below.
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary encode format ?-option value ...? data",
        dialects: Some(DialectSet::TCL86_PLUS),
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary decode format ?-option value ...? data",
        dialects: Some(DialectSet::TCL86_PLUS),
    },
];

/// The three `format` names `binary encode`/`binary decode` accept as their
/// first (sub-index 0) argument — always a single atomic word, never a
/// list, so unlike `open`'s `access` argument this is safe to mark in
/// `closed_value_args`. Exhaustive per the Tcl 8.6–9.1 manpages (TIP 317
/// introduced exactly these three formats; none were added or removed
/// through 9.1).
const ENCODE_DECODE_FORMAT_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "base64",
        detail: "MIME/base64 text encoding. Uses mostly upper- and lower-case letters and digits, and can be rewrapped arbitrarily without losing information.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "hex",
        detail: "Each byte as a pair of hexadecimal digits. Encoding always produces lowercase; decoding accepts both cases.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "uuencode",
        detail: "Legacy Unix uuencode body encoding. Neither binary encode nor binary decode handles the surrounding begin/end header and footer lines.",
        ..ArgValue::DEFAULT
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "decode",
        arity: Arity::at_least(2),
        detail: "Decode base64/hex/uuencode-encoded text back into a binary string.",
        synopsis: "binary decode format ?-option value ...? data",
        pure: true,
        // S110 binary source: the return type marks the result a byte array —
        // tclsh 8.6.14-verified (`tcl::unsupported::representation
        // [binary decode hex 41ffc8]` → bytearray); 9.0 `BinaryDecodeHex`/
        // `64`/`Uu` (tclBinary.c) build the result with `Tcl_NewObj` +
        // `Tcl_SetByteArrayLength` likewise.
        return_type: Some(TclType::ByteArray),
        // `data` is arg 1 (sub-index 1: arg 0 is the `format` keyword, e.g.
        // `hex`/`base64`). It is text-friendly *encoded* data being
        // *decoded into* a byte array, so the expectation is String, not
        // ByteArray — the reverse of `encode` below. Reading it only takes
        // the string rep, which is generated *alongside* the existing intrep
        // (dual-porting), never in place of it — tclsh-verified:
        // `set d 4142; binary decode hex $d` leaves `d` an int. No shimmer.
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: false,
                transparent_from: &[],
            },
        )],
        // `format` (sub-index 0) is always one of these three literal words —
        // see `ENCODE_DECODE_FORMAT_VALUES`'s doc comment for why this is
        // safe to close, unlike `open`'s `access` argument.
        arg_values: &[(0, ENCODE_DECODE_FORMAT_VALUES)],
        closed_value_args: &[0],
        // `-strict` is documented only under decoding for every format
        // (base64/hex/uuencode); encoding takes no `-strict` option.
        options: const {
            &[OptionSpec {
                name: "-strict",
                value: OptionValue::flag(),
                detail: "Raise an error on input that deviates from the encoding instead of silently tolerating it: a non-base64 character for base64, whitespace for hex, or a non-standard line for uuencode.",
                ..OptionSpec::DEFAULT
            }]
        },
        // `binary encode`/`binary decode` added in Tcl 8.6 (TIP 317).
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        arity: Arity::at_least(2),
        detail: "Encode binary data as base64, hex, or uuencode text.",
        synopsis: "binary encode format ?-option value ...? data",
        pure: true,
        return_type: Some(TclType::String),
        // S110: reads `data` *as bytes*, installing the byte-array rep on it
        // in place — tclsh 8.6.14-verified (`set q héllo; binary encode hex
        // $q` leaves `q` a bytearray). 8.6 `BinaryEncodeHex`/`64`/`Uu`
        // (tclBinary.c) convert via `Tcl_GetByteArrayFromObj(objv[objc-1])`,
        // silently truncating characters > 0xFF (`UCHAR`); 9.0 (TIP 568) uses
        // `Tcl_GetBytesFromObj(interp, objv[objc-1], …)` which **errors**
        // ("expected byte sequence but character …") on an improper byte
        // sequence and only installs the rep when proper. The `value_arg`
        // models the option-less `binary encode <fmt> <data>` form; with
        // `-maxlen`/`-wrapchar` options the data shifts right and is
        // (conservatively) not tracked.
        byte_array_effect: ByteArrayEffect::Rebinarifies { value_arg: 1 },
        // `data` is arg 1 (sub-index 1: arg 0 is the `format` keyword).
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::ByteArray),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        // `format` (sub-index 0) is always one of these three literal words —
        // see `ENCODE_DECODE_FORMAT_VALUES`'s doc comment.
        arg_values: &[(0, ENCODE_DECODE_FORMAT_VALUES)],
        closed_value_args: &[0],
        // `-maxlen`/`-wrapchar` are documented only for base64 and uuencode
        // (`hex` takes no encoding options at all — noted in each option's
        // `detail`, since the registry does not model per-`format` option
        // sets). Neither option is present under decoding.
        options: const {
            &[
                OptionSpec {
                    name: "-maxlen",
                    value: OptionValue::Takes(OptionArg {
                        integer: Some(IntegerDomain::Any),
                        hint: "length",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Split the output into lines of at most length characters. Ignored for hex. base64: unlimited by default (no splitting). uuencode: 5-85, default 61.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-wrapchar",
                    value: OptionValue::value("character"),
                    detail: "Line separator used when -maxlen splits the output. Ignored for hex. base64: any character, default \"\\n\". uuencode: zero or more of tab/VT/FF/CR followed by an optional newline, default a single newline.",
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        // `binary encode`/`binary decode` added in Tcl 8.6 (TIP 317).
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "format",
        arity: Arity::at_least(1),
        detail: "Build a binary string from Tcl values, laid out by a cursor-driven format specification.",
        synopsis: "binary format formatString ?arg ...?",
        pure: true,
        // S110 binary source: the return type marks the result a byte array —
        // tclsh 8.6.14-verified (`tcl::unsupported::representation
        // [binary format c* {1 2}]` → bytearray). Not stamped `Rebinarifies`:
        // which value args the `a`/`A` cursors read as bytes depends on the
        // format string, so the in-place conversion (8.6
        // `Tcl_GetByteArrayFromObj(objv[arg])`; 9.0 `TclNarrowToBytes`, which
        // converts in place only for a *proper* byte sequence and otherwise
        // works on a truncated copy without erroring) is not modelled and a
        // damaged operand conservatively stays damaged.
        return_type: Some(TclType::ByteArray),
        // The format string is read via its string rep only — cached
        // alongside the intrep, so a list-typed format spec keeps its list
        // intrep (tclsh-verified). No shimmer.
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: false,
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(2),
        detail: "Parse fields out of a binary string into variables, using a cursor-driven format specification. Returns the number of variables successfully set.",
        synopsis: "binary scan string formatString ?varName ...?",
        return_type: Some(TclType::Int),
        // S110: reads `string` *as bytes*, installing the byte-array rep on
        // it in place — the documented fix for byte-array corruption (F5
        // K22406348: `binary scan $v c* -` re-binarifies `$v`). tclsh
        // 8.6.14-verified; 8.6 `BinaryScanCmd` (tclBinary.c) converts via
        // `Tcl_GetByteArrayFromObj(objv[1])` with silent `UCHAR` truncation
        // of characters > 0xFF, 9.0 (TIP 568) via
        // `Tcl_GetBytesFromObj(interp, objv[1], …)` which errors on them.
        // S110-damaged values only hold characters <= 0xFF (latin-1 range),
        // so the fix behaves identically in both versions.
        byte_array_effect: ByteArrayEffect::Rebinarifies { value_arg: 0 },
        // `binary scan` writes format-dependent values (`a` → string, `c`/`s`/
        // `i` → int, `f` → double, `@` → none) to its targets while returning
        // the *count* of conversions.  The targets are not the count, so they
        // must not be typed `Int` (issue #867).
        var_write_typing: VarWriteTyping::Destructured,
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::ByteArray),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                1,
                ArgTypeHint {
                    // The format string is read via its string rep only
                    // (dual-ported — intrep kept). No shimmer.
                    expected: Some(TclType::String),
                    shimmers: false,
                    transparent_from: &[],
                },
            ),
        ],
        arg_role_resolver: Some(binary_scan_arg_roles),
        ..SubCommand::DEFAULT
    },
];

/// D4-F2: `binary scan string formatString ?varName ...?` accepts
/// variable-name args from index 2 onward (the resolver receives the args
/// *after* the `scan` subcommand word: `string`, `format`, then the vars).
/// Resolve `VarWrite` dynamically so calls with arbitrarily many vars don't
/// false-fire W210 on the unmodelled tail.  Mirrors the plain `scan`
/// command's own resolver (`scan_arg_roles` in `scan_.rs`).
fn binary_scan_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    (2..args.len())
        .filter_map(|i| u8::try_from(i).ok().map(|i| (i, ArgRole::VarWrite)))
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "binary",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        traits: Traits::BYTE_COMPILED | Traits::CSE_CANDIDATE | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Manipulate binary data",
            synopsis: &[
                "binary format formatString ?arg arg ...?",
                "binary scan string formatString ?varName varName ...?",
                "binary encode format ?-option value ...? data",
                "binary decode format ?-option value ...? data",
                "binary subcommand ?arg ...?",
            ],
            snippet: "This command provides facilities for manipulating binary data. format builds a binary string from Tcl values and scan parses one back out into variables; both walk an imaginary cursor through the data driven by a shared formatString mini-language of type-count field specifiers (a/A/b/B/h/H/c/s/S/i/I/w/W/f/d/x/X/@, plus t/n/m native-byte-order and r/R/q/Q little/big-endian float forms since 8.5). encode and decode instead convert to and from a text encoding (base64, hex, or uuencode) and were added in Tcl 8.6. A Tcl \"binary string\" is simply one whose characters are all in the range \\u0000-\\u00FF.",
            source: "Tcl man page binary.n",
            examples: "set packed [binary format c3 {72 105 33}]\nbinary scan $packed c3 codes\n# codes is now \"72 105 33\"\n\nset hex [binary encode hex $packed]\nset back [binary decode hex $hex]\n# back is again the 3-byte binary string",
            return_value: "format, encode, and decode return the newly built value (format/decode a byte array, encode a text string); scan returns the number of variables it successfully set.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
