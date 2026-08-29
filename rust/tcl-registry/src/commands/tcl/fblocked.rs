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

//! `fblocked` — test whether the last input operation exhausted all available input.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "fblocked channel",
    ..FormSpec::DEFAULT
}];

/// Command spec for `fblocked`.
///
/// `fblocked` is a plain core builtin in every standard Tcl version 8.4
/// through 9.1 — its behaviour and one-argument arity never changed. What
/// changed is only how the *manpage* presents it: the 8.4/8.5/8.6 pages
/// (`channelId` parameter name) document the full behaviour and ship a
/// worked network-server example, while the 9.0/9.1 pages (`channel`
/// parameter name) shrink to a one-line pointer at `chan blocked` ("has
/// been superceded by the chan blocked command which supports the same
/// syntax and options") with no independent description, example, or
/// `SEE ALSO` beyond `chan` — a documentation simplification, not a
/// behavioural or arity change, so this is modelled as accurate prose in
/// the hover snippet rather than a version gate on the command or a second
/// form. `chan` (and so `chan blocked`) itself first exists in Tcl 8.5 —
/// there is no `chan.n` manpage under the 8.4 tree.
///
/// iRules bans `fblocked` (F5's restricted event runtime has no real
/// blocking-I/O model to query) — so its `dialects` group below is
/// `ALL_TCL`, which does not carry the `IRULES` bit and therefore never
/// intersects the bare `IRULES` availability mask. That exclusion is
/// correct and deliberate, not an oversight. No other modelled dialect
/// (Expect, Tk, the EDA vendor shells, iApps, incr Tcl) disables or
/// alters `fblocked`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fblocked",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Test whether the last input operation on a channel was blocked by exhausted input.",
            synopsis: &["fblocked channel"],
            snippet: "Returns 1 if the most recent input operation on channel (gets, read, and similar) returned less data than requested because all available input was exhausted, and 0 otherwise. For example, if gets is called on a non-blocking channel that currently has only three characters available and no end-of-line sequence, gets returns an empty string and a following fblocked call returns 1. This lets network servers process input line by line on non-blocking channels without stalling other connections while a line is still incomplete. chan blocked (Tcl 8.5 and later) is the ensemble-style equivalent with identical syntax; from Tcl 9.0 onward it is the form Tcl's own documentation recommends, though fblocked keeps working unchanged.",
            source: "Tcl fblocked(n)",
            examples: "fconfigure $chan -blocking 0 -buffering line\nfileevent $chan readable [list onReadable $chan]\n\nproc onReadable {chan} {\n    gets $chan line\n    if {[eof $chan]} {\n        close $chan\n    } elseif {![fblocked $chan]} {\n        puts \"got: $line\"\n    }\n}",
            return_value: "1 if the last input operation was blocked by exhausted input, 0 otherwise.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
