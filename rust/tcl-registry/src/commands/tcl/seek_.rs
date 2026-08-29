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

//! `seek` — set the access position for a channel.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

// Tcl 8.4's seek.n synopsis and body ("seek channelId offset ?origin?",
// the three start/current/end origins, the empty-string return, the
// not-seekable error) are byte-for-byte identical in 8.5 and 8.6 (bar a
// purely cosmetic "nonblocking" -> "non-blocking" spelling fix in 8.6's
// body text). From Tcl 9.0, seek.n itself shrinks to a two-line stub —
// "The seek command has been superceded by the chan seek command which
// supports the same syntax and options" — and its SYNOPSIS line renames
// the placeholder from `channelId` to `channel`; 9.1's page is otherwise
// byte-for-byte identical to 9.0's. That is a documentation consolidation,
// not a behavioural or arity change: chan.n's own "chan seek" entry
// documents exactly the same start/current/end origins, empty-string
// return, and not-seekable error that seek.n always has, and plain `seek`
// keeps working exactly as before in every version through 9.1 (no
// tclsh9.0/9.1 binary was available locally to double-check directly, but
// there is no documented behavioural delta to model — see the module doc
// on `spec()` below).
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "seek channelId offset ?origin?",
    ..FormSpec::DEFAULT
}];

/// `origin` (position 2)'s three legal values — present, unchanged, in
/// every fetched seek.n/chan.n from Tcl 8.4 through 9.1. Real tclsh
/// accepts any unique abbreviation down to a single letter (confirmed
/// empirically against tclsh 8.6: `seek $chan 2 s`, `seek $chan 2 c`, and
/// `seek $chan 0 e` all succeed identically to the full words, and `chan
/// seek` accepts the same abbreviations; `seek $chan 0 bogus` raises `bad
/// origin "bogus": must be start, current, or end`) — like `open`'s
/// `ACCESS_VALUES` and `close`'s `DIRECTION_VALUES`, a flat per-word
/// closed set can't express unique-prefix matching, so this is
/// completion/hover data only; `seek` has no `closed_value_args` entry
/// for this position. (`chan seek`'s own sibling subcommand spec in
/// `chan_.rs` currently *does* mark this position `closed_value_args`
/// with a comment claiming "exact match required" — that claim did not
/// hold up under the same empirical check; out of scope to fix here since
/// it lives in a different command's file.)
const ORIGIN_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "start",
        detail: "Offset bytes from the start of the underlying file or device. The default when origin is omitted. Any unique abbreviation down to \"s\" is also accepted.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "current",
        detail: "Offset bytes from the current access position; a negative offset moves backwards. Any unique abbreviation down to \"c\" is also accepted.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "end",
        detail: "Offset bytes from the end of the file or device; a negative offset places the position before EOF, a positive offset places it after EOF. Any unique abbreviation down to \"e\" is also accepted.",
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `seek`.
///
/// Arity, the three `origin` values, the empty-string return, and the
/// not-seekable error condition are unchanged across Tcl 8.4-9.1 — the
/// only version-sensitive fact is that seek.n's own manual page became a
/// stub pointing at chan.n from Tcl 9.0 onward (see the `FORMS` doc
/// comment above), which is prose for the hover snippet, not a dialect or
/// arity gate.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "seek",
        // Core Tcl command whose surface is `ALL_TCL` (no iRules row). Its
        // absence from F5 iRules — the sandboxed TMM interpreter has no real
        // filesystem/channel-seek support — falls straight out of that
        // omission: an `ALL_TCL` surface is simply absent under iRules, so no
        // separate disable list is needed — the established pattern for this
        // whole class of command (`open` is likewise `ALL_TCL`, banned from
        // iRules for the identical reason). No other dialect (iApps, tmsh, Tk,
        // Expect, the EDA shells, itcl) disables or overrides `seek`.
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            // Repositioning mutates the channel's access position (and,
            // per the manpage, flushes pending output and discards
            // buffered unread input) but never yields data to the caller,
            // so this is a write-only effect — mirrors `close`'s FileIo
            // entry; contrast `tell`, which only queries and is
            // `reads: true, writes: false`.
            reads: false,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Change the access position for an open channel.",
            synopsis: &["seek channelId offset ?origin?"],
            snippet: "channelId identifies an already-open channel: a Tcl standard channel (stdin, stdout, stderr), the result of open or socket, or a channel created by an extension. offset is an integer (which may be negative) counted in bytes, not characters — like tell, seek always operates in bytes, unlike read.\n\norigin selects what offset is relative to: start (the default) counts from the beginning of the file or device; current counts from the current access position, with a negative offset moving backwards; end counts from the end, with a negative offset landing before EOF and a positive offset after it. Any unique abbreviation of start/current/end down to a single letter is also accepted.\n\nBefore repositioning, seek flushes all buffered output for the channel — even in nonblocking mode — and discards any buffered, unread input. Applying seek to a channel whose underlying file or device does not support seeking (a pipe, and most sockets) is an error.\n\nFrom Tcl 9.0 the seek manual page documents itself purely as an alias for chan seek (\"has been superceded by the chan seek command which supports the same syntax and options\"), though seek keeps working exactly as described above in every version through 9.1. The chan ensemble's chan seek subcommand, added in Tcl 8.5, offers identical behaviour under a different spelling.",
            source: "Tcl seek(n)",
            examples: "# Get a file's size using seek + tell\nset f [open report.log r]\nseek $f 0 end\nset fileSize [tell $f]\nclose $f\n\n# Read just the last 10 bytes of a file\nset f [open data.bin r]\nfconfigure $f -translation binary\nseek $f -10 end\nset tail [read $f 10]\nclose $f",
            return_value: "Empty string.",
        }),
        forms: FORMS,
        arg_values: &[(2, ORIGIN_VALUES)],
        ..CommandSpec::DEFAULT
    }
}
