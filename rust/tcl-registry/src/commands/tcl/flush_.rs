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

//! `flush` — flush buffered output for a channel.

use crate::prelude::*;

// `channelId` is mandatory in every version's synopsis — 8.4/8.5/8.6's
// flush.n gives only `flush channelId`, and 9.0/9.1's abbreviated flush.n
// (see below) still gives only `flush channel`. There is no bare/optional
// form; a missing or extra argument is `wrong # args`, matching the
// `Arity::exact(1)` below. iRules disables `flush` outright (one of the
// K36322151 bans), so its `dialects` group below is `ALL_TCL`, which omits
// the `IRULES` bit and never intersects the bare `IRULES` availability
// mask — deliberate, not an oversight.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "flush channelId",
    ..FormSpec::DEFAULT
}];

/// Command spec for `flush`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "flush",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Flush buffered output for a channel.",
            synopsis: &["flush channelId"],
            // The blocking/nonblocking behaviour below is documented in
            // full on flush.n through 8.4, 8.5, and 8.6. From 9.0 onward
            // flush.n itself is reduced to a one-line pointer at chan(n)
            // ("The flush command has been superceded by the chan flush
            // command which supports the same syntax and options.") — that
            // is a documentation reorganisation, not a behaviour change:
            // `flush` itself is not removed or deprecated at the interpreter
            // level in 9.0/9.1, so the full behaviour is stated here as the
            // current truth for every version rather than only for 8.4-8.6.
            snippet: "channelId must identify an open channel that has been opened for writing — a Tcl standard channel (stdout or stderr), the result of open or socket, or a channel created by a Tcl extension. On a blocking channel, flush does not return until all buffered output has reached the channel; on a nonblocking channel, it may return before flushing completes, with the remainder flushed in the background as fast as the underlying file or device can absorb it. Since Tcl 8.5, chan flush offers the identical functionality through the chan ensemble; from Tcl 9.0 the flush manual page itself just redirects to chan flush, though flush remains a fully supported command in every version.",
            source: "Tcl flush(n)",
            examples: "puts -nonewline \"Please type your name: \"\nflush stdout\ngets stdin name\nputs \"Hello there, $name!\"",
            return_value: "Empty string on success.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
