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

//! `read` — read from a channel.
//!
//! Present in every Tcl version from 8.4 through 9.1 with the same two
//! forms and the same read semantics throughout: `read ?-nonewline?
//! channelId` reads to end-of-file (discarding one trailing newline when
//! `-nonewline` is given and the read genuinely reached end-of-file), and
//! `read channelId numChars` reads exactly `numChars` characters, or
//! fewer if the channel runs out first. The two forms are mutually
//! exclusive — `-nonewline` is only accepted when `numChars` is omitted.
//! 8.4's, 8.5's, and 8.6's own read.n document this in full (multi-byte
//! encoding, `-translation`, nonblocking behaviour, serial-port
//! caveats); Tcl 9.0/9.1's own read.n page is reduced to a one-line
//! pointer — "The read command has been superceded by the chan read
//! command which supports the same syntax and options" — a
//! documentation reorganisation, not a removal or a behaviour change:
//! `read` remains a fully supported, independent command in 9.0/9.1, and
//! the facts below for those two versions are drawn from chan.n's `read`
//! subcommand section (the shared implementation) rather than from
//! read.n itself — the same treatment `gets_.rs` / `puts_.rs` /
//! `flush_.rs` give their own 9.0+ manual-page stubs. The synopsis's
//! parameter name itself changes from `channelId` (8.4-8.6) to `channel`
//! (9.0/9.1) — cosmetic only, so `channelId` is kept here to match the
//! fuller, canonical wording every other version uses.
//!
//! One real behavioural addition *is* present from 9.0 on: chan.n
//! documents that when the channel's encoding profile is strict (the
//! TIP 656 default), `read` raises a POSIX EILSEQ error on invalid
//! encoding data instead of passing it through silently — absent from
//! every 8.4/8.5/8.6 fetch, where no such profile concept existed.
//! `read`'s own EILSEQ paragraph is more detailed than — and differs
//! from — `gets`'s (see `gets_.rs`): on a blocking channel the read
//! position advances to the invalid data and the successfully-decoded
//! prefix is returned via the `-data` key of the error's option
//! dictionary, while on a nonblocking channel the decoded prefix comes
//! back with no exception at all and the error is deferred to the next
//! read at that position.
//!
//! `dialects: Some(DialectSet::ALL_TCL)` on the command spec below is
//! deliberate, not an oversight: `read` is excluded from iRules, just
//! like `gets`/`flush`/`open`/`fconfigure` (K36322151) — its `ALL_TCL`
//! group carries no `IRULES` bit, so it never intersects the bare
//! `IRULES` availability mask and falls out by plain intersection, with
//! no disable list. No other modelled dialect (Expect, the EDA vendor
//! shells, F5 iApps/tmsh, Tk, incr Tcl) restricts or extends it.

use crate::prelude::*;

// Both forms are unchanged, word-for-word (bar the cosmetic
// channelId/channel rename noted in the module doc comment), across
// every fetched version, 8.4 through 9.1. `channelId` is kept — over
// 9.0/9.1's own `channel` rename — to match the fuller, canonical
// wording every other version uses, the same choice `gets_.rs` /
// `puts_.rs` / `flush_.rs` make.
const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "read ?-nonewline? channelId",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "read channelId numChars",
        dialects: None,
    },
];

/// Dynamic arg-role resolver: an optional leading `-nonewline` shifts
/// which raw-arg index is the channel — `read channelId`, `read
/// -nonewline channelId`, and `read channelId numChars` all place the
/// channel at whichever index immediately follows a literal
/// `-nonewline` first word, if present (unlike `puts_arg_roles` in
/// `puts_.rs`, the channel is mandatory in every `read` form, so no
/// argument-count gate is needed beyond "a word is actually there").
/// Declaring the channel position lets the taint sink's position-aware
/// filter (`sink_var_position_safe` in `tcl_compiler::taint`) recognise
/// a tainted channel handle as a non-dangerous argument generically,
/// with no `read`-by-name check in the compiler.
fn read_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let skip: u8 = u8::from(args.first() == Some(&"-nonewline"));
    if usize::from(skip) < args.len() {
        vec![(skip, ArgRole::Channel)]
    } else {
        Vec::new()
    }
}

/// Command spec for `read`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::BYTE_COMPILED | Traits::TAINT_SOURCE,
        // Positional count only: 1 (bare channelId, or -nonewline
        // skipped as a leading option leaving just channelId) or 2
        // (channelId numChars, with no leading option consumed). The
        // registry's arity checker skips a recognised *leading* option
        // (here, -nonewline, declared below) before counting positional
        // words, so this range can't by itself catch the invalid
        // combination `read -nonewline channelId numChars`: real Tcl
        // rejects it (the two forms are mutually exclusive per the
        // manpage), but with -nonewline skipped as a leading option the
        // remaining 2 positional words still fit this same 1..=2 range.
        // Unchanged across every fetched version, 8.4 through 9.1.
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(read_arg_roles),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        // `-nonewline` is documented identically — name, shape, and
        // behaviour — in every fetched version, 8.4 through 9.1.
        options: const {
            &[OptionSpec {
                name: "-nonewline",
                value: OptionValue::flag(),
                detail: "In the bare (no numChars) form, discard one trailing newline from the data read — but only when the read reached true end-of-file; has no effect on an early nonblocking return. Not valid together with numChars.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Read data from a channel, to end-of-file or a fixed character count.",
            synopsis: &["read ?-nonewline? channelId", "read channelId numChars"],
            snippet: "In the bare form, read reads all of the data from channelId up to the point the channel signals a failure — end-of-file, or, on a nonblocking channel, a blocked or other error condition. With -nonewline, a single trailing newline is discarded from what was read, but only when the read actually reached end-of-file; the switch has no effect when the command returns early, which happens only on a nonblocking channel that has run out of currently-available data. In the second form, read channelId numChars reads exactly numChars characters, or fewer if the channel runs out first; -nonewline is not accepted together with numChars. channelId must identify an open, readable channel — a Tcl standard channel (stdin), the result of open or socket, or a channel from an extension. If the channel uses a multi-byte encoding, the character count read may differ from the byte count consumed, and any trailing bytes that don't yet form a complete character are held back until one is available or end-of-file is reached; end-of-line sequences in the data are translated to newlines according to the channel's -translation setting (see fconfigure). On a nonblocking channel, read returns whatever data is already available rather than blocking for the rest. On a blocking serial-port channel, read can block indefinitely: the bare form until an end-of-file character arrives (see fconfigure -eofchar — forever, if none is configured), and the numChars form until exactly that many characters arrive; most serial-port code instead configures the channel nonblocking (fconfigure channelId -blocking 0) to sidestep both hazards. Since Tcl 8.5, chan read offers the identical functionality through the chan ensemble, and from Tcl 9.0 the read manual page itself just redirects there — though read remains a fully supported, independent command in every version. Starting in Tcl 9.0, a channel with the strict (default) encoding profile makes read raise a POSIX EILSEQ error on invalid encoding data instead of passing it through: on a blocking channel the position advances to the bad data and the successfully-decoded prefix is returned via the -data key of the error's option dictionary, while on a nonblocking channel the decoded prefix comes back with no exception and the error is deferred to the next read at that position.",
            source: "Tcl read(n)",
            examples: "set fl [open example.txt]\nset data [read $fl]\nclose $fl\nputs \"read [string length $data] characters\"\nset fl [open example.txt]\nset header [read $fl 4]\nclose $fl",
            return_value: "The characters read, as a string: everything up to end-of-file in the bare form, or up to numChars characters (fewer if the channel is shorter) in the second form. An empty string when no data is available, including at end-of-file.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
