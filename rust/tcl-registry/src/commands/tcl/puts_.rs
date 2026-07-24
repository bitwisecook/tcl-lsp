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

//! `puts` — write to a channel.
//!
//! Present in every Tcl version from 8.4 through 9.1 with the same write
//! semantics throughout: writes `string` to `channelId` (default
//! `stdout`), appending a newline unless `-nonewline` is given. 8.4's,
//! 8.5's, and 8.6's own puts.n document this in full (translation,
//! internal buffering, blocking vs. nonblocking behaviour); Tcl 9.0/9.1's
//! own puts.n page is reduced to a one-line pointer — "The puts command
//! has been superceded by the chan puts command which supports the same
//! syntax and options" — a documentation reorganisation, not a removal or
//! a behaviour change: `puts` remains a fully supported, independent
//! command in 9.0/9.1 (`chan puts` itself only exists from 8.5, when the
//! `chan` ensemble was introduced by TIP 208 — confirmed against the TIP
//! document itself, whose spec table lists `chan puts ; # puts`
//! explicitly, not just inferred from the chan.n fetches), and the facts
//! below for those two versions are drawn from the fuller 8.4-8.6 text —
//! the same treatment `gets_.rs` / `flush_.rs` give their own 9.0+
//! manual-page stubs. The synopsis's parameter name itself changes from
//! `channelId` (8.4-8.6) to `channel` (9.0/9.1) — cosmetic only, so
//! `channelId` is kept here to match the fuller, canonical wording every
//! other version uses.
//!
//! One real behavioural addition *is* present from 9.0 on: chan.n's own
//! `puts` subcommand section documents that `puts` now raises a POSIX
//! EILSEQ error when the channel's encoding profile is `strict` (the
//! TIP 656 default per `encoding(n)`: "strict being the default if the
//! -profile is not specified") and the string being written cannot be
//! represented in the channel's configured encoding — the write may be
//! partially completed before the error is raised. Absent from every
//! 8.4/8.5/8.6 fetch: `chan configure`'s `-profile` option itself does
//! not exist before 9.0 (confirmed missing from both the 8.5 and 8.6
//! `chan.n` option tables), so no encoding-profile concept existed for
//! `puts` to be gated by.
//!
//! `dialects: Some(DialectSet::ALL_TCL)` on the command spec below is
//! deliberate, not an oversight: `puts` is excluded from iRules, just
//! like `gets`/`flush`/`open`/`fconfigure` (K36322151) — its `ALL_TCL`
//! group carries no `IRULES` bit, so it never intersects the bare
//! `IRULES` availability mask and falls out by plain intersection, with
//! no disable list involved. No other modelled dialect (Expect, the EDA
//! vendor shells, F5 iApps/tmsh, Tk, incr Tcl) restricts or extends it.

use crate::prelude::*;

// The single invocation form is unchanged, word-for-word (bar the
// cosmetic channelId/channel rename noted above), across every fetched
// version, 8.4 through 9.1.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "puts ?-nonewline? ?channelId? string",
    dialects: None,
}];

/// Dynamic arg-role resolver: `puts` takes one positional arg (`string`,
/// writing to stdout) or two (`channelId string`); the optional leading
/// `-nonewline` flag shifts which raw-arg index is the channel. Declaring
/// the channel position lets the taint sink's position-aware filter
/// (`sink_var_position_safe` in `tcl_compiler::taint`) recognise a tainted
/// channel handle as a non-dangerous argument generically, with no
/// `puts`-by-name check in the compiler.
fn puts_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let skip: u8 = u8::from(args.first() == Some(&"-nonewline"));
    if args.len() - usize::from(skip) >= 2 {
        vec![(skip, ArgRole::Channel)]
    } else {
        Vec::new()
    }
}

/// Command spec for `puts`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "puts",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::TAINT_SINK,
        // Positional count only (1 = string, 2 = channelId string) — the
        // registry's arity checker skips a recognised *leading* option
        // (here, `-nonewline`, declared below) before counting positional
        // words, so the flag itself is not part of this range. Unchanged
        // across every fetched version, 8.4 through 9.1.
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(puts_arg_roles),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        // `-nonewline` is documented identically — name, shape, and
        // behaviour — in every fetched version, 8.4 through 9.1.
        options: const {
            &[OptionSpec {
                name: "-nonewline",
                value: OptionValue::flag(),
                detail: "Suppress the newline puts normally appends after string.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Write a string to a channel (stdout by default).",
            synopsis: &["puts ?-nonewline? ?channelId? string"],
            snippet: "Writes string to channelId, appending a newline unless -nonewline is given. channelId must be an open, writable channel — a Tcl standard channel (stdout or stderr), the result of open or socket, or a channel created by a Tcl extension — and defaults to stdout when omitted. Newline characters are translated to the channel's platform-specific end-of-line sequence according to its -translation setting; see fconfigure. Tcl buffers output internally, so text written may not reach the underlying file or device immediately — use flush to force it out. On a blocking channel, puts blocks once the output buffer fills, until the operating system accepts the buffered data; on a nonblocking channel it never blocks, instead buffering arbitrarily large amounts of data in the background, so pair nonblocking output with the event loop and fileevent to avoid unbounded memory growth. Since Tcl 8.5, chan puts offers the identical functionality through the chan ensemble, and from Tcl 9.0 the puts manual page itself just redirects there — though puts remains a fully supported, independent command in every version. Starting in Tcl 9.0, if the channel's encoding profile is strict (the default), puts raises a POSIX EILSEQ error when the string being written cannot be represented in the channel's configured encoding, instead of silently substituting a fallback character for the unencodable data (as the non-default tcl8/replace profiles do); the write may be partially completed before the error is raised.",
            source: "Tcl puts(n)",
            examples: "puts \"Hello, World!\"\nputs -nonewline \"Hello, \"\nputs \"World!\"\nputs stderr \"Hello, World!\"\nset chan [open my.log a]\nputs $chan [clock format [clock seconds]]\nclose $chan",
            return_value: "Empty string on success.",
        }),
        // Tainted data reaching `puts` output → T101.
        taint_output_sink: Some("T101"),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
