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

//! `fileevent` — execute a script when a channel becomes readable or writable.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "fileevent channelId readable ?script?",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "fileevent channelId writable ?script?",
        dialects: None,
    },
];

/// The `readable`/`writable` positional argument (index 1). Exact match
/// required — no documented abbreviation, unlike e.g. `chan close`'s
/// direction — and identical wording in every fetched manual page
/// (8.4/8.5/8.6/9.0/9.1), so no per-value `min_tcl` gate. Mirrors `chan
/// event`'s own `EVENT_VALUES` (`chan_.rs`), since the two commands share
/// this exact positional vocabulary — the 9.0/9.1 `fileevent.n` page
/// itself says so: "has been superceded by the chan event command which
/// supports the same syntax and options".
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

/// Command spec for `fileevent`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileevent",
        // Universal across every standard-Tcl version this registry models
        // (8.4 through 9.1 — confirmed present, with unchanged arity and
        // readable/writable keywords, in all five fetched manual pages) and
        // every other dialect except iRules. Note the SYNOPSIS placeholder
        // itself is *not* byte-for-byte identical: 8.4/8.5/8.6 name it
        // `channelId`, while 9.0/9.1 (whose fileevent.n is a one-line stub
        // pointing to `chan event`) name it `channel` — cosmetic only, no
        // calling-convention change, and `channelId` is kept below in
        // FORMS/hover to match `chan_.rs`'s own `chan event channelId event
        // ?script?` synopsis. F5's TMM interpreter strips the whole event-loop /
        // channel-config command surface from the sandboxed interpreter
        // (the K36322151 bans), which excludes `fileevent` there — but that
        // is deliberately modelled as a *subtractive* entry in
        // `IRULES_DISABLED_COMMANDS` (tcl-dialect/src/profile.rs), not as a
        // narrower `dialects` value here: folding it into e.g.
        // `TCL84 | IRULES` would re-admit the very command the profile
        // bans (see that file's own warning against doing so).
        dialects: None,
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel), (2, ArgRole::Body)],
        // "The script for a file event is executed at global level
        // (outside the context of any Tcl procedure)" — Tcl man page
        // fileevent.n, worded identically in the 8.4, 8.5, and 8.6 manual
        // pages (the 9.0/9.1 pages drop the prose but point to `chan
        // event`, which documents the same thing). SSA must not treat the
        // script as sharing the caller's frame — the same reasoning
        // `after`'s deferred-script forms use (`after_.rs`).
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        arg_values: &[(1, EVENT_VALUES)],
        closed_value_args: &[1],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            // Omitting `script` reads back the currently registered
            // handler (or "" if none is set); giving `script` — even an
            // empty one, which deletes the handler — writes it. The
            // static spec can't see which form a given call uses, so both
            // are declared, matching `chan event`'s own side effects.
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Execute a script when a channel becomes readable or writable.",
            synopsis: &[
                "fileevent channelId readable ?script?",
                "fileevent channelId writable ?script?",
            ],
            snippet: "Binds channelId's readable or writable state to script: script is evaluated every time the channel reaches that state, until the handler is replaced or deleted. There is at most one readable and one writable handler per channel per interpreter at a time; giving a new script for the same channel/state replaces the old one. Omitting script returns the currently registered script, or an empty string if none is set; giving script as an empty string deletes the handler. A handler is also deleted automatically when its channel is closed or its interpreter is deleted, and — to stop a buggy handler looping — whenever it raises an error.\n\nA channel counts as readable when unread data is available on the underlying device or already sitting in the input buffer (except immediately after a `gets` that found no complete line), or when it has hit EOF or an error condition; script must check for EOF itself, or an infinite loop can result. A channel counts as writable when at least one byte can be written without blocking, or on an error condition. Event-driven I/O works best on a channel placed in nonblocking mode with `fconfigure $chan -blocking 0` — on a blocking channel, `puts`/`gets`/`read` can themselves block and prevent any events (including this one) from being processed. Even on a readable channel, a single blocking byte-oriented `read` is not always guaranteed to succeed without blocking (multi-byte encodings, compression, and encryption transforms are documented exceptions); test `eof` only after an actual read attempt, since the EOF flag is set only once such an attempt has reached the end of the stream, not before.\n\nThe script runs at global level, outside the context of any Tcl procedure: local variables from the caller's proc are not visible inside it. Capture caller state with a command prefix instead, e.g. `fileevent $chan readable [list GetData $chan]`. An error raised while it runs is reported through interp bgerror (Tcl 8.5 and later; via the global bgerror proc directly on Tcl 8.4, which predates the interp bgerror subcommand).\n\nFrom Tcl 9.0 the official manual page documents this functionality under chan event (identical syntax and behaviour) instead of repeating it here; the standalone fileevent command itself is unaffected and keeps working exactly as described above.",
            source: "Tcl fileevent(n)",
            examples: "proc GetData {chan} {\n    if {[gets $chan line] >= 0} {\n        puts $line\n    }\n    if {[eof $chan]} {\n        close $chan\n    }\n}\n\nfconfigure $chan -blocking 0 -buffering line -translation crlf\nfileevent $chan readable [list GetData $chan]",
            return_value: "An empty string when script is given (registers, replaces, or deletes the handler); the currently registered script, or an empty string if none, when script is omitted.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
