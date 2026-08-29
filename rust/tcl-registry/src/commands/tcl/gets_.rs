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

//! `gets` — read a line from a channel.
//!
//! Present in every Tcl version from 8.4 through 9.1 with an identical
//! `gets channelId ?varName?` synopsis and the same read semantics
//! throughout. Tcl 9.0/9.1's own gets.n page is reduced to a one-line
//! pointer — "The gets command has been superceded by the chan gets
//! command which supports the same syntax and options" — a
//! documentation reorganisation, not a removal or a behaviour change:
//! gets remains a fully supported, independent command in 9.0/9.1, and
//! the facts below for those two versions are drawn from chan.n's
//! `gets` subcommand section (the shared implementation) rather than
//! from gets.n itself, the same treatment `fconfigure_.rs` gives its
//! own 9.0+ manual-page stub. One real behavioural addition *is*
//! present from 9.0 on: chan.n documents that gets now raises a POSIX
//! EILSEQ error when the channel's encoding profile is strict (the
//! TIP 656 default) and the input contains invalid encoding data —
//! absent from every 8.4/8.5/8.6 fetch, where no such profile concept
//! existed and malformed input was never rejected this way.
//!
//! The `ALL_TCL` `dialects` group on the command spec below is
//! deliberate, not an oversight: iRules bans `gets` outright (one of the
//! K36322151 bans, alongside `open`/`fconfigure`/`flush`/…), and
//! `ALL_TCL` does not carry the `IRULES` bit, so the spec never
//! intersects the bare `IRULES` availability mask — the same pattern
//! `flush_.rs` / `cd.rs` / `fconfigure_.rs` document for their own bans.
//! No other modelled dialect (Expect, the EDA vendor shells, F5
//! iApps/tmsh, Tk, incr Tcl) restricts or extends `gets`.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

// `channelId` is mandatory in every version's synopsis; `varName` is
// always optional — there is no bare/no-argument form, matching
// `Arity::new(1, 2)` below. 8.4/8.5/8.6's gets.n documents the full
// behaviour under the parameter name `channelId`; 9.0/9.1's one-line
// stub keeps a synopsis too, but renames the parameter to `channel` —
// a cosmetic rename only (mirrors flush.n's own 9.0+ synopsis
// reduction), so `channelId` is kept here to match the fuller,
// canonical wording every other version uses.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "gets channelId ?varName?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `gets`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "gets",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED | Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::VarWrite)],
        assigns_variable_at: Some(1),
        // `gets` has no single "the" return type: without `varName` the
        // result is the line itself (a String); with `varName` the
        // result is the character count instead (an Int, or -1 — see
        // the hover snippet). The registry's `return_type` has no
        // per-form axis yet (`CommandForm` carries no return-type
        // field of its own). A prior revision of this comment modelled
        // the two-arg form's Int count instead (reasoning that it is
        // the more idiomatic `while {[gets $chan line] >= 0} {...}`
        // usage) — reverted after it broke real taint propagation
        // through the equally-realistic one-arg `set x [gets $chan]`
        // pattern (`tcl-compiler`'s `t100_fires_for_open_tainted_filename`
        // and 50+ other taint tests, all of which read `gets`'s own
        // result as a string). Until `return_type` gains a per-form
        // axis, String is the safer of the two imprecise choices here.
        return_type: Some(TclType::String),
        // The two-arg `gets chan varName` form writes the read *line* (a
        // String) to its target while returning the character *count* (an
        // Int).  Type the target as the line it always receives, not the
        // count (issue #867).
        var_write_typing: VarWriteTyping::Fixed(TclType::String),
        side_effects: &[
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
        ],
        hover: Some(HoverSnippet {
            summary: "Read a line from a channel.",
            synopsis: &["gets channelId ?varName?"],
            snippet: "Reads the next line from channelId — every character up to (but not including) the end-of-line sequence — and discards the end-of-line sequence itself. channelId must identify an open, readable channel: stdin, the result of open or socket, or a channel created by a Tcl extension. Without varName, the line read is returned as the command's result. With varName, the line is stored in that variable instead and the result is the character count read. If end-of-file is reached partway through a line, whatever was read so far is still returned (or stored) as a partial line. On a nonblocking channel with no complete line yet available, gets consumes no input and returns early: an empty string when varName is omitted, or -1 (leaving varName empty) when it is given. Because a genuinely blank line, end-of-file, and this nonblocking not-yet-available case can all read as an empty string when varName is omitted, use eof and fblocked to tell them apart. Since Tcl 8.5, chan gets offers the identical functionality through the chan ensemble, and from Tcl 9.0 the gets manual page itself just redirects there — though gets remains a fully supported, independent command in every version. Starting in Tcl 9.0, if the channel's encoding profile is strict (the default), gets raises a POSIX EILSEQ error instead of silently passing through malformed encoding data; the channel's read position is left unchanged, so the encoding can be adjusted and the read retried.",
            source: "Tcl gets(n)",
            examples: "set chan [open some.file.txt]\nset lineNumber 0\nwhile {[gets $chan line] >= 0} {\n    puts \"[incr lineNumber]: $line\"\n}\nclose $chan",
            return_value: "Without varName: the line read, or an empty string at end-of-file or when no full line is yet available on a nonblocking channel. With varName: the character count of the line read, or -1 at end-of-file or on an incomplete nonblocking read.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
