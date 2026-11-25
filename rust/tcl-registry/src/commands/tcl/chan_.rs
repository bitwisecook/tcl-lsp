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
use crate::channel_options::CONFIGURE_OPTIONS;
use crate::channel_protocols::ChannelCallbackProtocol;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "chan subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// `chan copy`'s options.
const COPY_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-size",
        value: OptionValue::value("size"),
        detail: "Maximum number of bytes (or characters, if the channels' encodings differ) to copy before stopping. Without it, chan copy reads until end of file.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
        effect: None,
    },
    OptionSpec {
        name: "-command",
        value: ChannelCallbackProtocol::Copy.option("callback"),
        detail: "Run the copy in the background and return immediately; callback is invoked on completion with the byte/character count appended, plus an error-message argument if the copy failed. inputChan/outputChan are switched to non-blocking automatically; an active event loop (vwait or Tk) is required to drive it.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
        effect: None,
    },
];

/// `chan puts`'s `-nonewline` flag.
const PUTS_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-nonewline",
    value: OptionValue::flag(),
    detail: "Suppress the trailing newline that chan puts otherwise appends.",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
    effect: None,
}];

/// `chan read`'s `-nonewline` flag.
const READ_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-nonewline",
    value: OptionValue::flag(),
    detail: "Trim a trailing newline from the data read. Only meaningful when reading to end of file (i.e. without numChars).",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
    effect: None,
}];

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

/// `chan seek`'s `origin` positional argument (index 2). Real tclsh
/// accepts any unique abbreviation of the three words down to a single
/// letter (`chan seek $c 0 s`), and raises `bad origin "bogus": must be
/// start, current, or end` for anything else — so the set is closed, but
/// matched by unique prefix (`arg_values_accept_prefix`), exactly as
/// `chan close`'s `direction` is.  The bare `seek` spec in `seek_.rs`
/// records the empirical check behind this.
const SEEK_ORIGIN_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "start",
        detail: "offset bytes from the start of the file or device. Default when origin is omitted. Any unique abbreviation down to \"s\" is also accepted.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "current",
        detail: "offset bytes from the current access position. Any unique abbreviation down to \"c\" is also accepted.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "end",
        detail: "offset bytes from the end of the file or device. Any unique abbreviation down to \"e\" is also accepted.",
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
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "close",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::new(1, 2),
        arity_windows: const {
            &[crate::arity::ArityWindow {
                lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("8.6"),
                arity: Arity::exact(1),
            }]
        },
        detail: "Close a channel, or (Tcl 8.6+) half-close one direction of a bidirectional channel.",
        synopsis: "chan close channelId ?direction?",
        // Same `Tcl_CloseObjCmd` machinery as the bare `close` (tclIOCmd.c
        // — `chan close` is an alias for it): destroys the channel and
        // errors on an already-closed handle, so `catch {chan close $h}`
        // is the documented fire-and-forget idiom the W302 suppression
        // keys off.
        destructive: true,
        arg_roles: &[(0, ArgRole::Channel)],
        arg_values: &[(1, super::close_::DIRECTION_VALUES)],
        closed_value_args: &[1],
        arg_values_accept_prefix: true,
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::stepped(1, Arity::UNLIMITED, 2).with_also_exact(2),
        detail: "Get or set channel configuration options; each channel type may add its own options beyond the generic set (see e.g. socket(n), open(n)).",
        synopsis: "chan configure channelId ?-option value ...?",
        arg_roles: &[(0, ArgRole::Channel)],
        options: CONFIGURE_OPTIONS,
        channel_configuration: Some(ChannelConfigurationSpec::EncodingTranslation),
        literal_argument_validator: Some(crate::channel_options::validate_chan_configuration),
        option_prefix_words: 1,
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "copy",
        evaluation_contexts: Some(
            crate::evaluation_context::EvaluationContextSpec::DEFERRED_GLOBAL,
        ),
        arity: Arity::stepped(2, 6, 2),
        detail: "Copy data between channels, blocking until complete unless -command is given.",
        synopsis: "chan copy inputChan outputChan ?-size size? ?-command callback?",
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::Channel)],
        options: COPY_OPTIONS,
        option_prefix_words: 2,
        return_type: Some(TclType::Int),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::exact(2),
        detail: "Create a reflected channel implemented by a Tcl command handler (see refchan(n)).",
        synopsis: "chan create mode cmdPrefix",
        arg_values: &[(0, CREATE_MODE_VALUES)],
        return_type: Some(TclType::Channel),
        command_prefixes: &[(1, ChannelCallbackProtocol::Reflected.appended_arity())],
        callback_protocols: &[(1, ChannelCallbackProtocol::Reflected)],
        literal_argument_validator: Some(crate::channel_options::validate_create_modes),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eof",
        surface: None,
        arity: Arity::exact(1),
        detail: "Test for end of file.",
        synopsis: "chan eof channelId",
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "event",
        body_execution: Some(crate::body_execution::BodyExecutionSpec::Deferred(
            crate::body_execution::DeferredExecutionSpec::global(
                crate::body_execution::DeferredTrigger::Channel,
            ),
        )),
        evaluation_contexts: Some(
            crate::evaluation_context::EvaluationContextSpec::DEFERRED_GLOBAL,
        ),
        surface: None,
        arity: Arity::new(2, 3),
        detail: "Set up a channel event handler for \"readable\" or \"writable\".",
        synopsis: "chan event channelId event ?script?",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::Channel), (2, ArgRole::Body)],
        arg_values: &[(1, EVENT_VALUES)],
        closed_value_args: &[1],
        // Compatibility fields agree with the deferred entry contract.
        body_kind: BodyKind::Structural,
        traits: Traits::DEFERS_BODY,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "flush",
        surface: None,
        arity: Arity::exact(1),
        detail: "Flush buffered output.",
        synopsis: "chan flush channelId",
        mutator: true,
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "gets",
        surface: None,
        traits: Traits::TAINT_SOURCE.union(Traits::UNCONDITIONAL_VARIABLE_WRITE),
        arity: Arity::new(1, 2),
        detail: "Read a line.",
        synopsis: "chan gets channelId ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::VarWrite)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::GETS),
        side_effects: crate::knowledge::Knowledge::Partial(&[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                ..SideEffect::DEFAULT
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::new(0, 1),
        detail: "List open channels, optionally filtered by a glob pattern.",
        synopsis: "chan names ?pattern?",
        arg_roles: &[(0, ArgRole::Pattern)],
        pattern_type: Some(PatternType::Glob),
        return_type: Some(TclType::List),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pending",
        arity: Arity::exact(2),
        detail: "Return the number of bytes currently buffered internally for input or output.",
        synopsis: "chan pending mode channelId",
        arg_roles: &[(1, ArgRole::Channel)],
        arg_values: &[(0, PENDING_MODE_VALUES)],
        closed_value_args: &[0],
        return_type: Some(TclType::Int),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pipe",
        // Absent from the 8.5 chan.n synopsis and body; present from 8.6
        // on (confirmed identical text in 8.6.18, 9.0.4, and 9.1b0).
        surface: Some(SpecSurface::TCL86_PLUS),
        arity: Arity::exact(0),
        detail: "Create a standalone connected pipe (Tcl 8.6+); returns its two channels.",
        synopsis: "chan pipe",
        return_type: Some(TclType::List),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pop",
        // Absent from the 8.5 chan.n synopsis and body; present from 8.6 on.
        surface: Some(SpecSurface::TCL86_PLUS),
        arity: Arity::exact(1),
        detail: "Remove the topmost channel transformation (Tcl 8.6+); equivalent to chan close if none remain.",
        synopsis: "chan pop channelId",
        mutator: true,
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "push",
        // Absent from the 8.5 chan.n synopsis and body; present from 8.6 on.
        surface: Some(SpecSurface::TCL86_PLUS),
        arity: Arity::exact(2),
        detail: "Add a channel transformation (Tcl 8.6+; see transchan(n)); returns a handle to it.",
        synopsis: "chan push channelId cmdPrefix",
        mutator: true,
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Channel),
        command_prefixes: &[(1, ChannelCallbackProtocol::Transform.appended_arity())],
        callback_protocols: &[(1, ChannelCallbackProtocol::Transform)],
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "puts",
        arity: Arity::new(1, 3),
        arg_role_layout_resolver: Some(super::puts_::puts_layout_roles),
        arg_role_resolver_roles: &[ArgRole::Channel],
        detail: "Write to a channel (default stdout).",
        synopsis: "chan puts ?-nonewline? ?channelId? string",
        mutator: true,
        options: PUTS_OPTIONS,
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
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
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "seek",
        surface: None,
        arity: Arity::new(2, 3),
        detail: "Set access position.",
        synopsis: "chan seek channelId offset ?origin?",
        mutator: true,
        arg_roles: &[(0, ArgRole::Channel)],
        arg_values: &[(2, SEEK_ORIGIN_VALUES)],
        closed_value_args: &[2],
        arg_values_accept_prefix: true,
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tell",
        surface: None,
        arity: Arity::exact(1),
        detail: "Return current position.",
        synopsis: "chan tell channelId",
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "truncate",
        arity: Arity::new(1, 2),
        detail: "Truncate a channel.",
        synopsis: "chan truncate channelId ?length?",
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isbinary",
        arity: Arity::exact(1),
        // `chan isbinary` was added in Tcl 9.0 — absent in the 8.5/8.6.x
        // series (confirmed absent from both fetched pages).
        surface: Some(SpecSurface::TCL90_PLUS),
        detail: "Test whether the channel is binary (iso8859-1 encoding, empty -eofchar, lf translation).",
        synopsis: "chan isbinary channelId",
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "postevent",
        literal_argument_validator: Some(crate::channel_options::validate_postevent_modes),
        arity: Arity::exact(2),
        detail: "Post an event to a reflected channel created with chan create.",
        synopsis: "chan postevent channelId eventSpec",
        arg_roles: &[(0, ArgRole::Channel)],
        arg_values: &[(1, POSTEVENT_VALUES)],
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "chan",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::Unresolved,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("chan"),
        traits: Traits::BYTE_COMPILED,
        // `chan` is a Tcl 8.5+ ensemble: no chan.n manual page and no `chan`
        // entry in the 8.4 command index (confirmed: the 8.4 URL 404s and the
        // 8.4 command index lists only the pre-ensemble standalone commands it
        // later wraps). Narrower than `ALL_TCL`, and — since iRules
        // availability is explicit per spec (§9 of the dialect-profile model),
        // a version-gated spec is excluded from iRules purely because its
        // non-`IRULES` surface never intersects iRules' bare `IRULES` mask
        // (iRules is pinned to an 8.4 base), the same way `dict`/`lmap` are —
        // this also correctly removes `chan` from iRules, which already
        // separately excludes the standalone commands it wraps (`open`,
        // `gets`, `fconfigure`, `flush`, `seek`, `tell`, `eof`).
        surface: Some(SpecSurface::TCL85_PLUS),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        options: CONFIGURE_OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Read, write and manipulate channels.",
            synopsis: &["chan subcommand ?arg ...?"],
            snippet: "Unified interface for channel operations (blocked, close, configure, copy, create, eof, event, flush, gets, names, pending, pipe, pop, postevent, push, puts, read, seek, tell, truncate, isbinary), superseding the older standalone close/eof/fconfigure/fcopy/fileevent/flush/gets/puts/read/seek/tell commands, which remain as compatibility aliases. chan pipe/pop/push need Tcl 8.6+; chan isbinary and the -profile/-nodelay/-keepalive/-inputmode configure options need Tcl 9.0+.",
            source: "Tcl chan(n)",
            examples: "chan configure stdout -buffering line\nchan puts -nonewline $sock $payload\nchan flush $sock\nset line [chan gets $sock]\nchan close $sock",
            return_value: "Depends on the subcommand — see each subcommand's detail.",
        }),
        forms: FORMS,
        side_effects: crate::knowledge::Knowledge::declared(SIDE_EFFECTS),
        ..CommandSpec::DEFAULT
    }
}
