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

//! `fconfigure` — set and get options on a channel.
//!
//! Present in every Tcl version from 8.4 through 9.1 (unlike the `chan`
//! ensemble, which is 8.5+ — see `chan_.rs`). Tcl 9.0/9.1's own
//! `fconfigure.n` page has been reduced to a two-line stub — "The
//! `fconfigure` command has been superceded by the `chan configure`
//! command which supports the same syntax and options" — so the 9.0/9.1
//! facts captured below come from `chan.n`'s `configure` subcommand
//! section (confirmed identical option surface) rather than from
//! `fconfigure.n` itself for those two versions.
//!
//! The `ALL_TCL` `dialects` group on the command spec below is
//! deliberate, not an oversight: iRules bans `fconfigure`, and `ALL_TCL`
//! does not carry the `IRULES` bit, so this spec never intersects the
//! bare `IRULES` availability mask — the same pattern `chan_.rs`
//! documents for `chan`.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "fconfigure channelId ?optionName? ?value ...?",
    ..FormSpec::DEFAULT
}];

/// `-buffering`'s values — always a single flat word in every fetched
/// version (8.4 through 9.1, via `fconfigure.n`/`chan.n`), so this is safe
/// to mark `closed: true`.
const BUFFERING_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "full",
        detail: "Buffer output until the internal buffer fills or flush is invoked. Default for non-terminal channels.",
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

/// `-translation`'s flat mode values — completion/hover data only
/// (`closed: false` on the option): the option also accepts a
/// `{inTranslation outTranslation}` two-element list for a read-write
/// channel in every fetched version, 8.4 through 9.1 alike — confirmed via
/// `chan.n`'s synopsis and prose for 9.0/9.1 (`fconfigure.n` itself is a
/// stub there), which document the list form with wording byte-identical
/// to 8.4–8.6's `fconfigure.n`/`chan.n`. Unlike `-eofchar`'s two-element
/// form (removed at 9.0 — see that option's `detail`), `-translation`'s
/// list form was never removed. See the `-translation` `OptionSpec`
/// below; a flat per-word enum can't validate the list form without
/// misreporting it as an invalid value (the same list pitfall `open`'s
/// `ACCESS_VALUES` documents).
const TRANSLATION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "auto",
        detail: "Default. On input, lf/cr/crlf are all accepted and mapped to \\n. On output, the platform's native end-of-line sequence is used (crlf for sockets on every platform and for Windows files, lf for Unix files).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "lf",
        detail: "Unix-style: a bare linefeed marks end of line, on both input and output. No translation occurs.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "cr",
        detail: "Macintosh-style: a bare carriage return marks end of line, on both input and output.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "crlf",
        detail: "Windows/network-style: carriage-return-linefeed marks end of line, on both input and output.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "binary",
        detail: "No end-of-line translation is performed (internally identical to lf, and reported as lf when queried). Also clears -eofchar and sets -encoding — documented as the binary pseudo-encoding on fconfigure.n through Tcl 8.6, and as iso8859-1 (a byte-for-byte passthrough, same effect) on chan.n's chan configure from Tcl 8.6 on and on every 9.0/9.1 source, fully configuring the channel for binary I/O.",
        ..ArgValue::DEFAULT
    },
];

/// `-profile`'s values (`encoding(n)` PROFILES section) — Tcl 9.0+ only:
/// absent from the `-eofchar`/`-translation`-only option list on every
/// 8.4/8.5/8.6 `fconfigure.n`/`chan.n` fetch, present with these exact
/// three members on both the 9.0.4 and 9.1b0 `chan.n` (`fconfigure.n`
/// itself is a stub for 9.0/9.1). `strict` is the default when `-profile`
/// is not set.
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

/// `-inputmode`'s values, documented on `open.n` (serial-port and Windows
/// console channel options), not on `fconfigure.n`/`chan.n` itself — Tcl
/// 9.0+ only: `grep -c inputmode` on the fetched 8.4/8.5/8.6 `open.n` pages
/// is 0, and it is present, unchanged, on both 9.0.4 and 9.1b0. `reset` is
/// set-only.
const INPUTMODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "normal",
        detail: "Line-oriented input with standard terminal/console editing enabled.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "password",
        detail: "Non-echoing input (terminal/console editing still enabled); typed characters are not written to the terminal except newlines.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "raw",
        detail: "All keyboard input is handed to Tcl with no terminal/console processing or echo at all.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "reset",
        detail: "Set-only: restores the terminal/console to the state it was in when the channel was opened.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `fconfigure`.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-blocking",
        value: OptionValue::boolean(),
        detail: "Whether I/O on the channel may block the process indefinitely. Must be a proper boolean; channels are blocking by default. Placing a channel in nonblocking mode affects gets, read, puts, flush, and close, and requires an active Tcl event loop (vwait, or Tcl_DoOneEvent) to drive it.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
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
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-buffersize",
        value: OptionValue::value("size"),
        detail: "Integer buffer size in bytes for buffers subsequently allocated for this channel. Tcl 8.4 and 8.5 require a value between 10 and 1,000,000; Tcl 8.6 lowers the minimum to 1; the Tcl 9.0/9.1 documentation states only the 1,000,000-byte ceiling, without restating a minimum.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-encoding",
        value: OptionValue::value("encoding"),
        detail: "Character encoding used to convert the channel's bytes to/from Tcl's internal string representation for reading and writing, e.g. shiftjis for a Japanese file. Defaults to the platform- and locale-dependent system encoding (see encoding system). Tcl 8.5/8.6's chan.n documents binary itself as a legal special encoding-name value for pure-binary data (matching fconfigure.n's own recommendation in every 8.4-8.6 source); Tcl 9.0's chan.n drops binary from its -encoding description (listing only \"the named encodings returned by encoding names\") and recommends iso8859-1 for binary data instead — see the binary -translation value below for the same shift.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-eofchar",
        value: OptionValue::value("char"),
        detail: "A character that signals end-of-file when encountered on input. Tcl 8.5 onward documents an acceptable range of \\x01–\\x7f (setting a value outside it is an error); Tcl 8.4's documentation states no such range restriction. Tcl 8.4 through 8.6 document that the character is also written on output when the channel closes, and that the default is the empty string everywhere except Windows file reads, where it defaults to Control-Z (\\x1a); Tcl 9.0/9.1's chan.n (fconfigure.n itself is a stub for those two versions) narrows the description to input-signalling only, stating simply that the default is 'no special end of file character marker' without restating the output-write behaviour or the Windows exception. Tcl 8.4 through 8.6 additionally accept a {inChar outChar} two-element list to set independent input/output eofchars on a read-write channel, returned as a two-element list when queried; Tcl 9.0 removed the two-element form — only a single value is accepted and it applies to both directions.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-translation",
        value: OptionValue::Takes(OptionArg {
            values: TRANSLATION_VALUES,
            hint: "mode",
            ..OptionArg::DEFAULT
        }),
        detail: "End-of-line translation mode; also accepts a {inTranslation outTranslation} two-element list to set input and output translation independently on a read-write channel (returned as a two-element list when queried). See the value list for per-mode behaviour and defaults.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-profile",
        value: OptionValue::Takes(OptionArg {
            values: PROFILE_VALUES,
            closed: true,
            hint: "profile",
            ..OptionArg::DEFAULT
        }),
        detail: "Encoding profile controlling how conversion errors on this channel's -encoding are handled (see PROFILES in encoding(n)). Defaults to strict.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    // Socket / terminal channel-type options, Tcl 9.0+. Settable only on
    // the matching channel type; documented on socket.n / open.n rather
    // than fconfigure.n/chan.n, since they belong to that channel driver
    // rather than to the generic option set.
    OptionSpec {
        name: "-nodelay",
        value: OptionValue::boolean(),
        detail: "TCP_NODELAY on a socket channel (Tcl 9.0+, TIP 344): disables Nagle's algorithm so small writes are sent immediately instead of being coalesced and delayed. Boolean value. Documented on socket(n).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-keepalive",
        value: OptionValue::boolean(),
        detail: "SO_KEEPALIVE on a socket channel (Tcl 9.0+, TIP 344): enables periodic keepalive probes on an otherwise idle connection. Boolean value. Documented on socket(n).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-inputmode",
        value: OptionValue::Takes(OptionArg {
            values: INPUTMODE_VALUES,
            closed: true,
            hint: "mode",
            ..OptionArg::DEFAULT
        }),
        detail: "Interactive input mode of a serial channel (Unix) or a console channel (Windows stdin/stdout) — Tcl 9.0+. Setting it to anything other than normal arranges for the terminal/console to be automatically reset when the channel is closed. Documented on open(n).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fconfigure",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::BYTE_COMPILED | Traits::CONFIGURES_CHANNEL | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        options: OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Set and get options on a channel.",
            synopsis: &[
                "fconfigure channelId",
                "fconfigure channelId optionName",
                "fconfigure channelId optionName value ?optionName value ...?",
            ],
            snippet: "With no optionName or value, returns a list of alternating option names and values for the channel. With optionName alone, returns that option's current value. With one or more optionName/value pairs, sets each option and returns the empty string. -blocking, -buffering, -buffersize, -encoding, -eofchar, and -translation apply to every channel; -profile is Tcl 9.0+; -nodelay/-keepalive only apply to socket channels (Tcl 9.0+) and -inputmode only to serial/console channels (Tcl 9.0+). Each channel type may also add its own options beyond this generic set — see socket(n) and open(n). Superseded by chan configure (Tcl 8.5+), which accepts the same syntax and options; fconfigure itself remains available as a compatibility form.",
            source: "Tcl fconfigure(n)",
            examples: "fconfigure stdout -buffering none\nfconfigure $sock -blocking 0\nset settings [fconfigure $chan]\nfconfigure $f -translation binary -eofchar {}",
            return_value: "A list of alternating option/value pairs, a single option's value, or the empty string, depending on how many arguments follow channelId.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
