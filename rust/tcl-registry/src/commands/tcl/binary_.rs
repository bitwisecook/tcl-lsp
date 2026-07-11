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

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary format formatString ?arg arg ...?",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary scan string formatString ?varName varName ...?",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary encode format ?-option value ...? data",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary decode format ?-option value ...? data",
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "decode",
        arity: Arity::at_least(2),
        detail: "Decode binary data.",
        synopsis: "binary decode format data",
        pure: true,
        return_type: Some(TclType::ByteArray),
        // `data` is arg 1 (sub-index 1: arg 0 is the `format` keyword, e.g.
        // `hex`/`base64`). It is text-friendly *encoded* data being
        // *decoded into* a byte array, so the shimmer-sensitive expectation
        // is String, not ByteArray — the reverse of `encode` below.
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
            },
        )],
        // `binary encode`/`binary decode` added in Tcl 8.6 (TIP 317).
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        arity: Arity::at_least(2),
        detail: "Encode binary data.",
        synopsis: "binary encode format data",
        pure: true,
        return_type: Some(TclType::String),
        // `data` is arg 1 (sub-index 1: arg 0 is the `format` keyword).
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::ByteArray),
                shimmers: true,
            },
        )],
        // `binary encode`/`binary decode` added in Tcl 8.6 (TIP 317).
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "format",
        arity: Arity::at_least(1),
        detail: "Format values into a binary string.",
        synopsis: "binary format formatString ?arg ...?",
        pure: true,
        return_type: Some(TclType::ByteArray),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(2),
        detail: "Parse a binary string.",
        synopsis: "binary scan string formatString ?varName ...?",
        return_type: Some(TclType::Int),
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
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
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
/// false-fire W210 on the unmodelled tail.  Mirrors the `binary scan`
/// resolver.
fn binary_scan_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    (2..args.len())
        .filter_map(|i| u8::try_from(i).ok().map(|i| (i, ArgRole::VarWrite)))
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "binary",
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
            snippet: "This command provides facilities for manipulating binary data. The principal operations are inserting values into a binary string and extracting values from a binary string.",
            source: "Tcl man page binary.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
