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
//!
//! `chan` itself is a Tcl 8.5+ ensemble (absent from 8.4 — there is no
//! `chan.n` manual page and no `chan` entry in the 8.4 command index, only
//! the standalone `close`/`eof`/`fconfigure`/`fcopy`/`fileevent`/`flush`/
//! `gets`/`open`/`puts`/`read`/`seek`/`tell` commands it later wraps), so
//! the command-level `dialects` gate below is `TCL85_PLUS`, not `None`.
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
    dialects: None,
}];

/// `-translation`'s flat mode values — completion/hover data only
/// (`closed: false` on the option): the option also accepts a
/// `{inTranslation outTranslation}` two-element list for a read-write
/// channel, which a flat per-word enum can't validate without misreporting
/// the list as an invalid value (the same list pitfall `open`'s
/// `ACCESS_VALUES` documents).
const TRANSLATION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "auto",
        detail: "Default. On input, lf/cr/crlf are all accepted and mapped to \\n. On output, the platform's native end-of-line sequence is used.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "lf",
        detail: "Unix-style: a bare linefeed marks end of line, on both input and output.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "cr",
        detail: "A bare carriage return marks end of line, on both input and output.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "crlf",
        detail: "Windows/network-style: carriage-return-linefeed marks end of line, on both input and output.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "binary",
        detail: "No end-of-line translation. Also clears -eofchar and sets -encoding to iso8859-1 (Tcl 8.6+; Tcl 8.5 instead set -encoding to the pseudo-encoding \"binary\"), fully configuring the channel for byte-transparent I/O.",
        ..ArgValue::DEFAULT
    },
];

/// `-buffering`'s values — always a single flat word in every fetched
/// version (no list form), so this is safe to mark `closed: true`.
const BUFFERING_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "full",
        detail: "Buffer until the internal buffer fills or chan flush is invoked. Default for non-terminal channels.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "line",
        detail: "Flush automatically whenever a newline is written. Default for terminal-like channels, and for stdin/stdout.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "none",
        detail: "Flush automatically after every write. Default for stderr.",
        ..ArgValue::DEFAULT
    },
];

/// `-profile`'s values (`encoding(n)` PROFILES section) — Tcl 9.0+ only
/// (absent from every 8.5/8.6 chan.n fetch; present, with these exact three
/// members, in both 9.0.4 and 9.1b0). `strict` is the default when
/// `-profile` is not set.
const PROFILE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "strict",
        detail: "Default. Any conversion error raises an exception (Unicode-conformant behaviour).",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "tcl8",
        detail: "Lenient, Tcl 8.6-compatible byte mapping: invalid bytes are mapped to their numerically equivalent code points instead of erroring.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "replace",
        detail: "Invalid bytes/characters are substituted with a replacement character instead of erroring.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
];

/// `-inputmode`'s values (TIP 160, `open(n)`) — Tcl 9.0+ only (absent from
/// every 8.5/8.6/9.0-less fetch of both chan.n and open.n; landed with the
/// rest of the "8.7" feature set folded into 9.0 since 8.7 itself was never
/// released).
const INPUTMODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "normal",
        detail: "Line-oriented input with standard terminal editing enabled.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "password",
        detail: "Non-echoing input (terminal editing still enabled); typed characters are not written to the terminal except newlines.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "raw",
        detail: "All keyboard input is handed to Tcl with no terminal processing or echo at all.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "reset",
        detail: "Set-only: restores the terminal to the state it was in when the channel was opened.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
];

/// Shared option table for `chan configure` — same shape as `fconfigure`
/// options. `-blocking`/`-buffering`/`-buffersize`/`-encoding`/`-eofchar`/
/// `-translation` are documented on chan.n itself and present in every
/// fetched version (8.5 through 9.1); `-profile` (TIP 656) and the
/// socket/terminal options `-nodelay`/`-keepalive` (TIP 344, documented on
/// socket.n) and `-inputmode` (TIP 160, documented on open.n) are Tcl 9.0+
/// — confirmed absent from the 8.5/8.6 chan.n and socket.n/open.n pages and
/// present, unchanged, in both 9.0.4 and 9.1b0.
static CONFIGURE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-blocking",
        value: OptionValue::value("boolean"),
        detail: "Whether I/O on the channel may block the process indefinitely. Channels are blocking by default; nonblocking mode affects chan gets/read/puts/flush/close and requires the event loop (vwait/update) to drive it.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffering",
        value: OptionValue::Takes(OptionArg {
            values: BUFFERING_VALUES,
            closed: true,
            hint: "mode",
            ..OptionArg::DEFAULT
        }),
        detail: "full, line, or none — see the value list for the per-mode default.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-buffersize",
        value: OptionValue::value("size"),
        detail: "Buffer size in bytes for buffers subsequently allocated for this channel; capped at 1,000,000.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-encoding",
        value: OptionValue::value("encoding"),
        detail: "Character encoding used to convert the channel's bytes to/from Tcl's internal string representation; defaults to the platform/locale system encoding (encoding system). Use iso8859-1 for byte-transparent binary data (Tcl 8.5 instead used the pseudo-encoding \"binary\" for this; removed in Tcl 9.0).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-eofchar",
        value: OptionValue::value("char"),
        detail: "A single character (\\x01-\\x7f) that signals end-of-file on input; the empty string (the default) disables it. Tcl 8.5/8.6 additionally accepted a {inChar outChar} two-element list for a read-write channel and defaulted to Control-Z on Windows file reads; Tcl 9.0+ takes a single value only and always defaults to the empty string.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-translation",
        value: OptionValue::Takes(OptionArg {
            values: TRANSLATION_VALUES,
            hint: "mode",
            ..OptionArg::DEFAULT
        }),
        detail: "End-of-line translation mode; also accepts a {inTranslation outTranslation} list to set input/output independently. See the value list for per-mode behaviour and defaults.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-profile",
        value: OptionValue::Takes(OptionArg {
            values: PROFILE_VALUES,
            closed: true,
            hint: "profile",
            ..OptionArg::DEFAULT
        }),
        detail: "Encoding profile controlling how conversion errors on this channel are handled (see PROFILES in encoding(n)).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    // Socket / terminal channel-type options, all Tcl 9.0+ (TIPs 344 and
    // 160). Settable only on the matching channel type; documented on
    // socket.n / open.n rather than chan.n, since they belong to that
    // channel driver, not to chan configure's own generic option set.
    OptionSpec {
        name: "-nodelay",
        value: OptionValue::value("boolean"),
        detail: "TCP_NODELAY on a socket channel: disables Nagle's algorithm so small writes are sent immediately (Tcl 9.0+, TIP 344). Documented on socket(n).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-keepalive",
        value: OptionValue::value("boolean"),
        detail: "SO_KEEPALIVE on a socket channel (Tcl 9.0+, TIP 344). Documented on socket(n).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-inputmode",
        value: OptionValue::Takes(OptionArg {
            values: INPUTMODE_VALUES,
            closed: true,
            hint: "mode",
            ..OptionArg::DEFAULT
        }),
        detail: "Terminal input mode on a serial/console channel (Tcl 9.0+, TIP 160); Unix-only (Windows exposes the equivalent on console channels). Documented on open(n).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
];

/// `chan copy`'s options.
const COPY_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-size",
        value: OptionValue::value("size"),
        detail: "Maximum number of bytes (or characters, if the channels' encodings differ) to copy before stopping. Without it, chan copy reads until end of file.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::command_prefix_n("callback", AppendedArity::AtLeast(1)),
        detail: "Run the copy in the background and return immediately; callback is invoked on completion with the byte/character count appended, plus an error-message argument if the copy failed. inputChan/outputChan are switched to non-blocking automatically; an active event loop (vwait or Tk) is required to drive it.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

/// `chan puts`'s `-nonewline` flag.
const PUTS_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-nonewline",
    value: OptionValue::flag(),
    detail: "Suppress the trailing newline that chan puts otherwise appends.",
    dialects: None,
    aliases: &[],
    min_version: None,
}];

/// `chan read`'s `-nonewline` flag.
const READ_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-nonewline",
    value: OptionValue::flag(),
    detail: "Trim a trailing newline from the data read. Only meaningful when reading to end of file (i.e. without numChars).",
    dialects: None,
    aliases: &[],
    min_version: None,
}];

/// `chan close`'s `direction` positional argument (index 1) — Tcl 8.6+
/// only (absent from the 8.5 chan.n synopsis and body text; present from
/// 8.6 on). Real tclsh accepts any unique abbreviation of `read`/`write`
/// for this argument, so it's declared with `arg_values_accept_prefix`
/// rather than an exact-match `closed_value_args`.
const CLOSE_DIRECTION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "read",
        detail: "Half-close only the read side of a bidirectional channel.",
        min_tcl: Some(TclVersion::V8_6),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "write",
        detail: "Half-close only the write side of a bidirectional channel.",
        min_tcl: Some(TclVersion::V8_6),
        ..ArgValue::DEFAULT
    },
];

/// `chan pending`'s `mode` positional argument (index 0). Real tclsh
/// requires an exact match — the manual page says "mode is input or
/// output", unlike `chan close`'s direction, which explicitly documents
/// abbreviation.
const PENDING_MODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "input",
        detail: "Bytes currently buffered internally for reading.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "output",
        detail: "Bytes currently buffered internally for writing.",
        ..ArgValue::DEFAULT
    },
];

/// `chan event`'s `event` positional argument (index 1). Exact match
/// required (no documented abbreviation, unlike `chan close`'s direction).
const EVENT_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "readable",
        detail: "Fire script when the channel becomes readable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "writable",
        detail: "Fire script when the channel becomes writable.",
        ..ArgValue::DEFAULT
    },
];

/// `chan seek`'s `origin` positional argument (index 2). Exact match
/// required.
const SEEK_ORIGIN_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "start",
        detail: "offset bytes from the start of the file or device. Default when origin is omitted.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "current",
        detail: "offset bytes from the current access position.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "end",
        detail: "offset bytes from the end of the file or device.",
        ..ArgValue::DEFAULT
    },
];

/// `chan create`'s `mode` positional argument (index 0) — a Tcl list
/// containing "read" and/or "write", so (like `open`'s `ACCESS_VALUES`)
/// this is completion/hover data only, never `closed_value_args`: a
/// legitimate `{read write}` list value would otherwise be misreported as
/// invalid.
const CREATE_MODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "read",
        detail: "List form: the reflected channel supports chan read/chan gets.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "write",
        detail: "List form: the reflected channel supports chan puts/chan flush.",
        ..ArgValue::DEFAULT
    },
];

/// `chan postevent`'s `eventSpec` positional argument (index 1) — a Tcl
/// list containing "read" and/or "write" (at least one), same list shape
/// as `chan create`'s mode.
const POSTEVENT_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "read",
        detail: "List form: notify the reflected channel that it has become readable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "write",
        detail: "List form: notify the reflected channel that it has become writable.",
        ..ArgValue::DEFAULT
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "blocked",
        arity: Arity::exact(1),
        detail: "Test whether the last input operation would have blocked; only ever true on a channel configured non-blocking.",
        synopsis: "chan blocked channelId",
        pure: true,
        arg_roles: &[(0, ArgRole::Channel)],
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
        detail: "Close a channel, or (Tcl 8.6+) half-close one direction of a bidirectional channel.",
        synopsis: "chan close channelId ?direction?",
        // Same `Tcl_CloseObjCmd` machinery as the bare `close` (tclIOCmd.c
        // — `chan close` is an alias for it): destroys the channel and
        // errors on an already-closed handle, so `catch {chan close $h}`
        // is the documented fire-and-forget idiom the W302 suppression
        // keys off.
        destructive: true,
        arg_roles: &[(0, ArgRole::Channel)],
        arg_values: &[(1, CLOSE_DIRECTION_VALUES)],
        closed_value_args: &[1],
        arg_values_accept_prefix: true,
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
        detail: "Get or set channel configuration options; each channel type may add its own options beyond the generic set (see e.g. socket(n), open(n)).",
        synopsis: "chan configure channelId ?-option value ...?",
        arg_roles: &[(0, ArgRole::Channel)],
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
        detail: "Copy data between channels, blocking until complete unless -command is given.",
        synopsis: "chan copy inputChan outputChan ?-size size? ?-command callback?",
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::Channel)],
        options: COPY_OPTIONS,
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
        detail: "Create a reflected channel implemented by a Tcl command handler (see refchan(n)).",
        synopsis: "chan create mode cmdPrefix",
        arg_values: &[(0, CREATE_MODE_VALUES)],
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
        arg_roles: &[(0, ArgRole::Channel)],
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
        detail: "Set up a channel event handler for \"readable\" or \"writable\".",
        synopsis: "chan event channelId event ?script?",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::Channel), (2, ArgRole::Body)],
        arg_values: &[(1, EVENT_VALUES)],
        closed_value_args: &[1],
        // The script fires later, when the channel event occurs — a
        // different call frame than the one that registered it (same
        // reasoning as `after`'s `body_kind`, which see).
        body_kind: BodyKind::Structural,
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
        arg_roles: &[(0, ArgRole::Channel)],
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
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::VarWrite)],
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
        detail: "List open channels, optionally filtered by a glob pattern.",
        synopsis: "chan names ?pattern?",
        pure: true,
        arg_roles: &[(0, ArgRole::Pattern)],
        pattern_type: Some(PatternType::Glob),
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
        detail: "Return the number of bytes currently buffered internally for input or output.",
        synopsis: "chan pending mode channelId",
        pure: true,
        arg_roles: &[(1, ArgRole::Channel)],
        arg_values: &[(0, PENDING_MODE_VALUES)],
        closed_value_args: &[0],
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
        // Absent from the 8.5 chan.n synopsis and body; present from 8.6
        // on (confirmed identical text in 8.6.18, 9.0.4, and 9.1b0).
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::exact(0),
        detail: "Create a standalone connected pipe (Tcl 8.6+); returns its two channels.",
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
        // Absent from the 8.5 chan.n synopsis and body; present from 8.6 on.
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::exact(1),
        detail: "Remove the topmost channel transformation (Tcl 8.6+); equivalent to chan close if none remain.",
        synopsis: "chan pop channelId",
        mutator: true,
        arg_roles: &[(0, ArgRole::Channel)],
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
        // Absent from the 8.5 chan.n synopsis and body; present from 8.6 on.
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::exact(2),
        detail: "Add a channel transformation (Tcl 8.6+; see transchan(n)); returns a handle to it.",
        synopsis: "chan push channelId cmdPrefix",
        mutator: true,
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Channel),
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
        detail: "Write to a channel (default stdout).",
        synopsis: "chan puts ?-nonewline? ?channelId? string",
        mutator: true,
        options: PUTS_OPTIONS,
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
        options: READ_OPTIONS,
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
        arg_roles: &[(0, ArgRole::Channel)],
        arg_values: &[(2, SEEK_ORIGIN_VALUES)],
        closed_value_args: &[2],
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
        arg_roles: &[(0, ArgRole::Channel)],
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
        arg_roles: &[(0, ArgRole::Channel)],
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
        // `chan isbinary` was added in Tcl 9.0 — absent in the 8.5/8.6.x
        // series (confirmed absent from both fetched pages).
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Test whether the channel is binary (iso8859-1 encoding, empty -eofchar, lf translation).",
        synopsis: "chan isbinary channelId",
        pure: true,
        arg_roles: &[(0, ArgRole::Channel)],
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
        detail: "Post an event to a reflected channel created with chan create.",
        synopsis: "chan postevent channelId eventSpec",
        arg_roles: &[(0, ArgRole::Channel)],
        arg_values: &[(1, POSTEVENT_VALUES)],
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
        dialects: Some(DialectSet::TCL90_PLUS),
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
        // `chan` is a Tcl 8.5+ ensemble: no chan.n manual page and no
        // `chan` entry in the 8.4 command index (confirmed: the 8.4 URL
        // 404s and the 8.4 command index lists only the pre-ensemble
        // standalone commands it later wraps). Narrower than `ALL_TCL`,
        // and — per the iRules profile's subtractive design (§9 of the
        // dialect-profile model: version-gated specs are excluded from
        // iRules by the pinned-8.4-base axis, not by an
        // `IRULES_DISABLED_COMMANDS` entry, the same way `dict`/`lmap`
        // are) — this also correctly removes `chan` from iRules, which
        // already separately bans the standalone commands it wraps
        // (`open`, `gets`, `fconfigure`, `flush`, `seek`, `tell`, `eof`).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        options: CMD_OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Read, write and manipulate channels.",
            synopsis: &["chan subcommand ?arg ...?"],
            snippet: "Unified interface for channel operations (blocked, close, configure, copy, create, eof, event, flush, gets, names, pending, pipe, pop, postevent, push, puts, read, seek, tell, truncate, isbinary), superseding the older standalone close/eof/fconfigure/fcopy/fileevent/flush/gets/puts/read/seek/tell commands, which remain as compatibility aliases. chan pipe/pop/push need Tcl 8.6+; chan isbinary and the -profile/-nodelay/-keepalive/-inputmode configure options need Tcl 9.0+.",
            source: "Tcl chan(n)",
            examples: "chan configure stdout -buffering line\nchan puts -nonewline $sock $payload\nchan flush $sock\nset line [chan gets $sock]\nchan close $sock",
            return_value: "Depends on the subcommand — see each subcommand's detail.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
