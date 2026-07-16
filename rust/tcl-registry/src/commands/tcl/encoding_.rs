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
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-profile",
    value: OptionValue::value("profile"),
    detail: "Encoding profile (strict, tcl8, replace).",
    dialects: None,
    aliases: &[],
    min_version: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "encoding subcommand ?arg ...?",
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "convertfrom",
        traits: Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        detail: "Convert from specified encoding.",
        synopsis: "encoding convertfrom ?encoding? data",
        pure: true,
        return_type: Some(TclType::String),
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "strict",
                    detail: "Stop on conversion error. Unicode-conformant.",
                    min_tcl: None,
                },
                ArgValue {
                    value: "tcl8",
                    detail: "Map invalid bytes to equivalent code points. Tcl 8 compatible.",
                    min_tcl: None,
                },
                ArgValue {
                    value: "replace",
                    detail: "Replace invalid data with U+FFFD. Unicode-conformant.",
                    min_tcl: None,
                },
            ],
        )],
        is_unescape: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "convertto",
        arity: Arity::new(1, 2),
        detail: "Convert to specified encoding.",
        synopsis: "encoding convertto ?encoding? string",
        pure: true,
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
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "strict",
                    detail: "Stop on conversion error. Unicode-conformant.",
                    min_tcl: None,
                },
                ArgValue {
                    value: "tcl8",
                    detail: "Map invalid bytes to equivalent code points. Tcl 8 compatible.",
                    min_tcl: None,
                },
                ArgValue {
                    value: "replace",
                    detail: "Replace invalid data with U+FFFD. Unicode-conformant.",
                    min_tcl: None,
                },
            ],
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dirs",
        arity: Arity::any(),
        detail: "Manage encoding search path.",
        synopsis: "encoding dirs ?directoryList?",
        return_type: Some(TclType::List),
        // Added in Tcl 8.5.
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return list of available encodings.",
        synopsis: "encoding names",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        arity: Arity::new(0, 1),
        detail: "Get or set system encoding.",
        synopsis: "encoding system ?encoding?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "profiles",
        arity: Arity::exact(0),
        // `encoding profiles` was added in Tcl 9.0 — absent in 8.6.x.
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Return list of available profiles.",
        synopsis: "encoding profiles",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "user",
        arity: Arity::exact(0),
        // `encoding user` was added in Tcl 9.0 — absent in 8.6.x.
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Return user encoding.",
        synopsis: "encoding user",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "encoding",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Convert between character encodings.",
            synopsis: &[
                "encoding convertfrom ?-profile profile? ?encoding? data",
                "encoding convertto ?-profile profile? ?encoding? string",
                "encoding names",
                "encoding system ?encoding?",
                "encoding subcommand ?arg ...?",
            ],
            snippet: "Use `encoding names` to list available encodings.",
            source: "Tcl man page encoding.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
