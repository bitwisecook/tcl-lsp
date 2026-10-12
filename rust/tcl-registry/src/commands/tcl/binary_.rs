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
use tcl_dialect::model::SpecSurface;

const BINARY_FORMAT_LOOKUP: crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::binary",
        member: "format",
        slot: "::tcl::binary::format",
        command: "binary",
        prepended: &["format"],
    };
const BINARY_SCAN_LOOKUP: crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::binary",
        member: "scan",
        slot: "::tcl::binary::scan",
        command: "binary",
        prepended: &["scan"],
    };

// TclMakeEnsemble copies BasicMin1Arg/BasicMin2Arg onto these actual
// C8.6+ private workers. Earlier C releases keep the public monolithic token.
const fn binary_named_compilation(
    lookup: &'static crate::native_compilation::NativeCompilerImplementationLookup,
    arity: Arity,
) -> crate::native_compilation::NativeCompilationSpec {
    crate::native_compilation::NativeCompilationSpec {
        grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
            lookup,
            implementation_from: tcl_dialect::TclVersion::V8_6,
            hook_from: tcl_dialect::TclVersion::V8_6,
            arity,
        },
        operation: crate::SemanticOperationId::Invoke,
        body: crate::native_compilation::NativeBodyCompilation::Inherit,
    }
}

const fn encoding_path(
    member: &'static str,
    slot: &'static str,
    prepended: &'static [&'static str],
) -> [crate::native_compilation::NativeCompilerImplementationLookup; 2] {
    use crate::native_compilation::NativeCompilerImplementationLookup;
    [
        NativeCompilerImplementationLookup {
            ensemble: "binary",
            member: "encode",
            slot: "::tcl::binary::encode",
            command: "binary",
            prepended: &["encode"],
        },
        NativeCompilerImplementationLookup {
            ensemble: "::tcl::binary::encode",
            member,
            slot,
            command: "binary",
            prepended,
        },
    ]
}

static ENCODE_HEX_PATH: [crate::native_compilation::NativeCompilerImplementationLookup; 2] =
    encoding_path("hex", "::tcl::binary::encode::hex", &["encode", "hex"]);
static ENCODE_BASE64_PATH: [crate::native_compilation::NativeCompilerImplementationLookup; 2] =
    encoding_path(
        "base64",
        "::tcl::binary::encode::base64",
        &["encode", "base64"],
    );
static ENCODE_UU_PATH: [crate::native_compilation::NativeCompilerImplementationLookup; 2] =
    encoding_path(
        "uuencode",
        "::tcl::binary::encode::uuencode",
        &["encode", "uuencode"],
    );

/// Audited C8.6+ public and nested encoding ensemble dependencies.
pub const ENCODE_IMPLEMENTATION_PATHS: crate::native_handler_path::NativeHandlerLookupPaths =
    crate::native_handler_path::NativeHandlerLookupPaths {
        argument: 0,
        alternatives: &[
            crate::native_handler_path::NativeHandlerLookupPath {
                value: "hex",
                lookups: &ENCODE_HEX_PATH,
            },
            crate::native_handler_path::NativeHandlerLookupPath {
                value: "base64",
                lookups: &ENCODE_BASE64_PATH,
            },
            crate::native_handler_path::NativeHandlerLookupPath {
                value: "uuencode",
                lookups: &ENCODE_UU_PATH,
            },
        ],
    };

const DECODE_HEX_LOOKUP: crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::tcl::binary::decode",
        member: "hex",
        slot: "::tcl::binary::decode::hex",
        command: "binary",
        prepended: &["decode", "hex"],
    };
const DECODE_BASE64_LOOKUP: crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::tcl::binary::decode",
        member: "base64",
        slot: "::tcl::binary::decode::base64",
        command: "binary",
        prepended: &["decode", "base64"],
    };
const DECODE_UU_LOOKUP: crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::tcl::binary::decode",
        member: "uuencode",
        slot: "::tcl::binary::decode::uuencode",
        command: "binary",
        prepended: &["decode", "uuencode"],
    };

const NATIVE_NULL_COMPILATION: crate::native_compilation::NativeCompilationSpec =
    crate::native_compilation::NativeCompilationSpec {
        grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
        operation: crate::SemanticOperationId::Invoke,
        body: crate::native_compilation::NativeBodyCompilation::Inherit,
    };

const fn decoding_path(
    terminal: crate::native_compilation::NativeCompilerImplementationLookup,
) -> [crate::native_compilation::NativeCompilerImplementationLookup; 2] {
    [
        crate::native_compilation::NativeCompilerImplementationLookup {
            ensemble: "binary",
            member: "decode",
            slot: "::tcl::binary::decode",
            command: "binary",
            prepended: &["decode"],
        },
        terminal,
    ]
}
static DECODE_HEX_PATH: [crate::native_compilation::NativeCompilerImplementationLookup; 2] =
    decoding_path(DECODE_HEX_LOOKUP);
static DECODE_BASE64_PATH: [crate::native_compilation::NativeCompilerImplementationLookup; 2] =
    decoding_path(DECODE_BASE64_LOOKUP);
static DECODE_UU_PATH: [crate::native_compilation::NativeCompilerImplementationLookup; 2] =
    decoding_path(DECODE_UU_LOOKUP);

static ENCODE_HEX_COMPILER: crate::native_compilation::NativeCompilationSpec =
    binary_named_compilation(&ENCODE_HEX_PATH[1], Arity::exact(1));
static DECODE_HEX_COMPILER: crate::native_compilation::NativeCompilationSpec =
    binary_named_compilation(&DECODE_HEX_LOOKUP, Arity::new(1, 2));
static DECODE_BASE64_COMPILER: crate::native_compilation::NativeCompilationSpec =
    binary_named_compilation(&DECODE_BASE64_LOOKUP, Arity::new(1, 2));
static DECODE_UU_COMPILER: crate::native_compilation::NativeCompilationSpec =
    binary_named_compilation(&DECODE_UU_LOOKUP, Arity::new(1, 2));

const fn binary_compiler_path(
    compiler: &'static crate::native_compilation::NativeCompilationSpec,
    lookups: &'static [crate::native_compilation::NativeCompilerImplementationLookup],
) -> crate::native_compilation::NativeCompilationSpec {
    crate::native_compilation::NativeCompilationSpec {
        grammar: crate::native_compilation::NativeCompilationGrammar::WithImplementationPath {
            compiler,
            lookups,
            implementation_from: tcl_dialect::TclVersion::V8_6,
            monolithic_no_hook_before: false,
        },
        operation: crate::SemanticOperationId::Invoke,
        body: crate::native_compilation::NativeBodyCompilation::Inherit,
    }
}

// Pinned C8.6–9.1 tables independently author the compiler prerequisites:
// encode hex Basic1Arg; wrapped encoders NULL; every decoder Basic1Or2Arg.
static ENCODE_WORKERS: &[crate::spec::SubSubCommand] = &[
    crate::spec::SubSubCommand {
        name: "hex",
        options: Some(&[]),
        native_compilation: Some(binary_compiler_path(&ENCODE_HEX_COMPILER, &ENCODE_HEX_PATH)),
        ..crate::spec::SubSubCommand::DEFAULT
    },
    crate::spec::SubSubCommand {
        name: "base64",
        native_compilation: Some(binary_compiler_path(
            &NATIVE_NULL_COMPILATION,
            &ENCODE_BASE64_PATH,
        )),
        ..crate::spec::SubSubCommand::DEFAULT
    },
    crate::spec::SubSubCommand {
        name: "uuencode",
        native_compilation: Some(binary_compiler_path(
            &NATIVE_NULL_COMPILATION,
            &ENCODE_UU_PATH,
        )),
        ..crate::spec::SubSubCommand::DEFAULT
    },
];
static DECODE_WORKERS: &[crate::spec::SubSubCommand] = &[
    crate::spec::SubSubCommand {
        name: "hex",
        native_compilation: Some(binary_compiler_path(&DECODE_HEX_COMPILER, &DECODE_HEX_PATH)),
        ..crate::spec::SubSubCommand::DEFAULT
    },
    crate::spec::SubSubCommand {
        name: "base64",
        native_compilation: Some(binary_compiler_path(
            &DECODE_BASE64_COMPILER,
            &DECODE_BASE64_PATH,
        )),
        ..crate::spec::SubSubCommand::DEFAULT
    },
    crate::spec::SubSubCommand {
        name: "uuencode",
        native_compilation: Some(binary_compiler_path(&DECODE_UU_COMPILER, &DECODE_UU_PATH)),
        ..crate::spec::SubSubCommand::DEFAULT
    },
];

// Only `binary scan` ever reaches this: `format`/`encode`/`decode` are
// `pure: true` on their `SubCommand` entries, so the compiler's side-effect
// classifier (`classify_side_effects`) short-circuits on subcommand purity
// before it ever consults this command-level fallback — see
// `tcl-compiler/src/side_effects.rs`'s `sub.pure && !sub.mutator` branch.
// `scan` writes fresh values into its `varName` targets (it does not read
// their prior contents), matching `scan_.rs`'s identical declaration.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "binary format formatString ?arg arg ...?",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "binary scan string formatString ?varName varName ...?",
        ..FormSpec::DEFAULT
    },
    // `binary encode`/`binary decode` were added in Tcl 8.6 (TIP 317) — see
    // the matching gate on the `encode`/`decode` `SubCommand` entries below.
    FormSpec {
        synopsis: "binary encode format ?-option value ...? data",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "binary decode format ?-option value ...? data",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
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
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::HookFrom(
                tcl_dialect::TclVersion::V8_6,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        sub_subcommands: DECODE_WORKERS,
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
        surface: Some(SpecSurface::TCL86_PLUS),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::HookFrom(
                tcl_dialect::TclVersion::V8_6,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        sub_subcommands: ENCODE_WORKERS,
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsemblePathLeaf {
                lookup: &ENCODE_IMPLEMENTATION_PATHS,
                implementation_from: tcl_dialect::TclVersion::V8_6,
            },
        ),
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
        surface: Some(SpecSurface::TCL86_PLUS),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "format",
        native_compilation: Some(binary_named_compilation(
            &BINARY_FORMAT_LOOKUP,
            Arity::at_least(1),
        )),
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider: Some(crate::native_compilation::NormalValueLeafProvider::F5Binary),
                implementation_from: tcl_dialect::TclVersion::V8_6,
                lookup: &BINARY_FORMAT_LOOKUP,
            },
        ),
        completion: Some(crate::completion::CompletionDescriptor::exact(&[
            crate::completion::CompletionCode::Ok,
            crate::completion::CompletionCode::Error,
        ])),
        arity: Arity::at_least(1),
        detail: "Build a binary string from Tcl values, laid out by a cursor-driven format specification.",
        synopsis: "binary format formatString ?arg ...?",
        pure: true,
        // The cursor-driven field string: index 0 after the subcommand word,
        // family `Binary`.
        arg_roles: &[(0, ArgRole::FormatString)],
        format_string_type: Some(FormatType::Binary),
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
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::BINARY_FORMAT),
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
        native_compilation: Some(binary_named_compilation(
            &BINARY_SCAN_LOOKUP,
            Arity::at_least(2),
        )),
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::VariableOperands,
        ),
        // The match / conversion path is the only one that writes: a failed
        // `regexp`, and a `scan` or `binary scan` whose input runs out, leave
        // each remaining target's previous value in place and never create a
        // target that did not exist. Measured identical on tclsh 8.4.20,
        // 8.5.19, 8.6.18, 9.0.4 and 9.1b0. Without this the store feeding one
        // looked overwritten-before-read and O109 deleted it (#2051).
        traits: Traits::CONDITIONAL_VARIABLE_WRITE,
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
        // must not be typed `Int`.
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
        arg_role_count_resolver: Some(binary_scan_arg_roles),
        arg_role_resolver_roles: &[ArgRole::ScanFormat, ArgRole::VarWrite],
        format_string_type: Some(FormatType::Binary),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::destructure::BINARY_SCAN),
        ..SubCommand::DEFAULT
    },
];

/// `binary scan string formatString ?varName ...?` accepts
/// variable-name args from index 2 onward (the resolver receives the args
/// *after* the `scan` subcommand word: `string`, `format`, then the vars).
/// Resolve `VarWrite` dynamically so calls with arbitrarily many vars don't
/// false-fire W210 on the unmodelled tail.  Mirrors the plain `scan`
/// command's own resolver (`scan_arg_roles` in `scan_.rs`). Unknown input or
/// format values do not change this cardinality-only layout.
fn binary_scan_arg_roles(argument_count: usize) -> Vec<(u8, ArgRole)> {
    // Index 1 (after the `scan` subcommand word) is the field string itself;
    // marking it `ScanFormat` is what lets the LSP locate it without naming
    // `binary`.
    std::iter::once((1u8, ArgRole::ScanFormat))
        .chain(
            (2..argument_count)
                .filter_map(|i| u8::try_from(i).ok().map(|i| (i, ArgRole::VarWrite))),
        )
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "binary",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::HookFrom(
                tcl_dialect::TclVersion::V8_6,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("binary"),
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
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
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
        NativeCompilationSelection, NativeCompilationWordShape,
    };
    use crate::{InvocationArguments, InvocationDialect, InvocationWord};

    #[test]
    fn nested_codec_compilers_preserve_exact_registration_and_selector() {
        let command = spec();
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            for (direction, rows) in [("encode", ENCODE_WORKERS), ("decode", DECODE_WORKERS)] {
                let parent = command.resolve_subcommand(direction).unwrap();
                for worker in rows {
                    let arguments = [worker.name];
                    let selected = parent.nested_native_compilation(
                        InvocationArguments::literals(&arguments).with_dialect(dialect),
                    );
                    if version < tcl_dialect::TclVersion::V8_6 {
                        assert!(selected.is_none());
                        continue;
                    }
                    let selected = selected.expect("audited actual nested worker");
                    let null_hook = direction == "encode" && worker.name != "hex";
                    assert_eq!(
                        selected.compiler_hook_presence(dialect) == Some(false),
                        null_hook
                    );
                    let path = selected.implementation_prerequisites(dialect).unwrap();
                    assert_eq!(path.len(), 2);
                    assert_eq!(path[0].ensemble, "binary");
                    assert_eq!(path[0].member, direction);
                    assert_eq!(path[1].member, worker.name);
                    if !null_hook {
                        let lookup = selected.implementation_lookup(dialect).unwrap();
                        let words = crate::InvocationWords::literals(lookup.slot, &["DATA"]);
                        let selection = selected.select(
                            words,
                            &[NativeCompilationWordShape::Substituted],
                            Some(dialect),
                            NativeCompilationContext {
                                mode: NativeCompilationMode::BytecodeObject,
                                frame: NativeCompilationFrame::ProcedureCode,
                                ..NativeCompilationContext::default()
                            },
                        );
                        assert!(matches!(
                            selection,
                            NativeCompilationSelection::NamedInvocation { .. }
                        ));
                    }
                }
                for selector in ["h", "future"] {
                    assert!(
                        parent
                            .nested_native_compilation(
                                InvocationArguments::literals(&[selector]).with_dialect(dialect)
                            )
                            .is_none()
                    );
                }
                assert!(
                    parent
                        .nested_native_compilation(
                            InvocationArguments::structured(&[InvocationWord::Dynamic])
                                .with_dialect(dialect)
                        )
                        .is_none()
                );
            }
        }
        let jim = crate::model::ingress::resolve_environment("jim").unit_profile();
        let dialect = InvocationDialect::of_profile(jim);
        assert!(
            command
                .resolve_subcommand("encode")
                .unwrap()
                .nested_native_compilation(
                    InvocationArguments::literals(&["hex"]).with_dialect(dialect)
                )
                .is_none()
        );
    }
}
