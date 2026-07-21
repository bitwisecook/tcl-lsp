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

//! `chan` — channel operations.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "chan subcommand ?arg ...?",
}];

/// Shared option table for `chan configure` — same shape as
/// `fconfigure` options.  The Tcl 9.0+ entries (`-nodelay`,
/// `-keepalive`, `-inputmode`) carry their own dialect gate so
/// W004 fires when they're used on pre-9.0 dialects.
static CONFIGURE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-blocking",
        value: OptionValue::value("boolean"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffering",
        value: OptionValue::value("mode"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffersize",
        value: OptionValue::value("size"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-encoding",
        value: OptionValue::value("encoding"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-eofchar",
        value: OptionValue::value("chars"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-translation",
        value: OptionValue::value("mode"),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // Tcl 9.0+ socket / terminal options (TIPs 344 / 160).
    OptionSpec {
        name: "-nodelay",
        value: OptionValue::value("boolean"),
        detail: "Disable Nagle's algorithm on TCP sockets (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-keepalive",
        value: OptionValue::value("boolean"),
        detail: "Enable TCP keepalive on sockets (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-inputmode",
        value: OptionValue::value("mode"),
        detail: "Terminal input mode: normal/password/raw (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "blocked",
        arity: Arity::exact(1),
        detail: "Test whether last input exhausted all data.",
        synopsis: "chan blocked channelId",
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "close",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::new(1, 2),
        detail: "Close a channel.",
        synopsis: "chan close channelId ?direction?",
        // Same `Tcl_CloseObjCmd` machinery as the bare `close` (tclIOCmd.c
        // — `chan close` is an alias for it): destroys the channel and
        // errors on an already-closed handle, so `catch {chan close $h}`
        // is the documented fire-and-forget idiom the W302 suppression
        // keys off.
        destructive: true,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Get or set channel configuration.",
        synopsis: "chan configure channelId ?-option value ...?",
        options: CONFIGURE_OPTIONS,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "copy",
        arity: Arity::at_least(2),
        detail: "Copy data between channels.",
        synopsis: "chan copy inputChan outputChan ?-option value ...?",
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::exact(2),
        detail: "Create a channel from a Tcl command.",
        synopsis: "chan create mode cmdPrefix",
        return_type: Some(TclType::Channel),
        // `cmdPrefix` (index 1) is the reflected-channel handler, invoked as
        // `cmdPrefix subcommand channelId ?args...?` — every method appends at
        // least the subcommand and channel id ⇒ AtLeast(2).
        command_prefixes: &[(1, AppendedArity::AtLeast(2))],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eof",
        dialects: None,
        arity: Arity::exact(1),
        detail: "Test for end of file.",
        synopsis: "chan eof channelId",
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "event",
        dialects: None,
        arity: Arity::new(2, 3),
        detail: "Set up channel event handler.",
        synopsis: "chan event channelId event ?script?",
        return_type: Some(TclType::String),
        arg_roles: &[(2, ArgRole::Body)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "flush",
        dialects: None,
        arity: Arity::exact(1),
        detail: "Flush buffered output.",
        synopsis: "chan flush channelId",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "gets",
        dialects: None,
        traits: Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        detail: "Read a line.",
        synopsis: "chan gets channelId ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::new(0, 1),
        detail: "List open channels.",
        synopsis: "chan names ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pending",
        arity: Arity::exact(2),
        detail: "Return pending bytes.",
        synopsis: "chan pending mode channelId",
        pure: true,
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pipe",
        arity: Arity::any(),
        detail: "Create a connected pair of channels.",
        synopsis: "chan pipe",
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pop",
        arity: Arity::exact(1),
        detail: "Remove the top channel transformation.",
        synopsis: "chan pop channelId",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "push",
        arity: Arity::exact(2),
        detail: "Add a channel transformation.",
        synopsis: "chan push channelId cmdPrefix",
        mutator: true,
        return_type: Some(TclType::String),
        // `cmdPrefix` (index 1) is the transform handler, invoked as
        // `cmdPrefix subcommand handle ?args...?` (a reflected-transform
        // dispatch) — minimum appended is subcommand + handle ⇒ AtLeast(2).
        command_prefixes: &[(1, AppendedArity::AtLeast(2))],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "puts",
        arity: Arity::at_least(1),
        detail: "Write to a channel.",
        synopsis: "chan puts ?-nonewline? ?channelId? string",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "read",
        traits: Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        detail: "Read from a channel.",
        synopsis: "chan read channelId ?numChars?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "seek",
        dialects: None,
        arity: Arity::new(2, 3),
        detail: "Set access position.",
        synopsis: "chan seek channelId offset ?origin?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tell",
        dialects: None,
        arity: Arity::exact(1),
        detail: "Return current position.",
        synopsis: "chan tell channelId",
        pure: true,
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "truncate",
        arity: Arity::new(1, 2),
        detail: "Truncate a channel.",
        synopsis: "chan truncate channelId ?length?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isbinary",
        arity: Arity::exact(1),
        // `chan isbinary` was added in Tcl 9.0 — absent in the 8.6.x series.
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Test whether channel is binary.",
        synopsis: "chan isbinary channelId",
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "postevent",
        arity: Arity::exact(2),
        detail: "Post an event to a reflected channel.",
        synopsis: "chan postevent channelId eventSpec",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

/// Command-level `chan configure` options (the `chan configure`
/// form options) — surfaced at the command level for completion consistency.
const CMD_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-blocking",
        value: OptionValue::value("boolean"),
        detail: "Set blocking mode.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffering",
        value: OptionValue::value("mode"),
        detail: "Set buffering mode (full, line, none).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffersize",
        value: OptionValue::value("size"),
        detail: "Set buffer size in bytes.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-encoding",
        value: OptionValue::value("encoding"),
        detail: "Set character encoding.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-eofchar",
        value: OptionValue::value("chars"),
        detail: "Set end-of-file character(s).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-profile",
        value: OptionValue::value("profile"),
        detail: "Set encoding profile (strict, tcl8, replace).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-translation",
        value: OptionValue::value("mode"),
        detail: "Set line-ending translation mode.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "chan",
        traits: Traits::BYTE_COMPILED | Traits::CONFIGURES_CHANNEL | Traits::HAS_DESTRUCTIVE_OPS,
        dialects: None,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        options: CMD_OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Read, write and manipulate channels.",
            synopsis: &["chan subcommand ?arg ...?"],
            snippet: "Unified interface for channel operations (`configure`, `gets`, `puts`, `read`, etc.).",
            source: "Tcl man page chan.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
