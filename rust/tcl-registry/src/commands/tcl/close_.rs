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

//! `close` — close, or half-close, an open channel.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "close channelId",
        ..FormSpec::DEFAULT
    },
    // The two-argument half-close form was added in 8.6: the 8.4 and 8.5
    // close.n synopses are just `close channelId`, with no direction
    // argument documented (or accepted — the 8.4/8.5 DESCRIPTION never
    // mentions a second word). 9.0/9.1's close.n keeps the same
    // `close channel ?r(ead)|w(rite)?` synopsis while deferring to chan(n)
    // for prose, stating close "supports the same syntax and options".
    FormSpec {
        synopsis: "close channelId ?r(ead)|w(rite)?",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
];

/// `close`'s optional direction argument (position 1), Tcl 8.6+ only. Real
/// tclsh accepts any unique abbreviation of `read`/`write` down to a bare
/// `r`/`w` (confirmed empirically against tclsh 8.6: `close $chan re` and
/// `close $chan wri` both succeed; `close $chan R`, `close $chan READ`, and
/// `close $chan rd` all raise `bad direction "...": must be read or
/// write`) — a flat per-word closed set can't express unique-prefix
/// matching, so like `open`'s `ACCESS_VALUES` this is completion/hover data
/// only; `close` has no `closed_value_args` entry for this position.
/// `min_tcl` therefore never reaches W137 (which only inspects closed value
/// sets), so the 8.6 introduction is also stated as a [`Lifecycle`] — the
/// general version-gate rung, which matches any literal value word.
/// (`chan close`'s twin subcommand spec models the identical fact via
/// `arg_values_accept_prefix`, a field only `SubCommand` carries —
/// top-level `CommandSpec`, which `close` is, has no such flag.)
const DIRECTION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "read",
        detail: "Half-close only the read side of a bidirectional channel (socket or command pipeline). Any unique abbreviation down to \"r\" is also accepted.",
        min_tcl: Some(TclVersion::V8_6),
        lifecycle: Lifecycle::introduced_in("8.6"),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "write",
        detail: "Half-close only the write side of a bidirectional channel (socket or command pipeline). Any unique abbreviation down to \"w\" is also accepted.",
        min_tcl: Some(TclVersion::V8_6),
        lifecycle: Lifecycle::introduced_in("8.6"),
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `close`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        surface: Some(SpecSurface::ALL_TCL),
        // `FIRE_AND_FORGET_TEARDOWN`: `Tcl_CloseObjCmd` (tclIOCmd.c) unregisters
        // and frees the channel — a second `close` on the same handle errors
        // ("can not find channel named …"), which is why a bare
        // `catch {close $h}` is the documented fire-and-forget idiom the
        // W302 suppression keys off this trait.
        traits: Traits::BYTE_COMPILED | Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        // Closing a command-pipeline channel (`open |...`) also waits for the
        // child process(es) to finish and raises an error on an abnormal
        // exit, "similar to the exec command" — documented unchanged 8.4
        // through 9.1 (9.0/9.1's close.n defers to chan(n) for prose but
        // states it "supports the same syntax and options"). That process
        // interaction is real but is intentionally not modelled as a second
        // `SideEffectTarget::Process` entry here (contrast `open`'s pipe
        // form, which starts the process): the registry- and
        // compiler-level test suites (e.g.
        // `registry_commands::close_side_effects_differ_by_dialect`,
        // `side_effects_binding::classify_close_in_tcl_is_file_io`) assert
        // plain-Tcl `close` classifies as exactly one `FileIo` effect, and
        // `close` only ever *waits for* a process `open` already started
        // rather than spawning one itself.
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        arg_values: &[(1, DIRECTION_VALUES)],
        hover: Some(HoverSnippet {
            summary: "Close, or half-close, an open channel.",
            synopsis: &["close channelId", "close channelId ?r(ead)|w(rite)?"],
            snippet: "The single-argument form flushes all buffered output, discards buffered input, and closes the underlying file or device; channelId becomes unavailable for use. A blocking channel does not return until output is flushed; a nonblocking channel with unflushed output stays open and closes in the background once flushing completes. Closing a command-pipeline channel waits for its child process(es) and raises an error on an abnormal exit.\n\nFrom Tcl 8.6, an optional direction (read or write, or any unique abbreviation) half-closes just that side of a bidirectional channel; currently only sockets and command pipelines support this. A later plain close on an already half-closed channel finishes the job; half-closing an already-closed side, or the wrong side of a unidirectional channel, is an error.\n\nAlso from 8.6 (TIP 398), a nonblocking channel with unflushed output is no longer forced back to blocking mode at process exit, so a stalled peer can no longer hang shutdown; set the environment variable TCL_FLUSH_NONBLOCKING_ON_EXIT to a nonzero value to restore the pre-8.6 behaviour.\n\nFrom Tcl 9.0 the close manual page documents itself purely as an alias for chan close (\"has been superceded by the chan close command which supports the same syntax and options\"), though close keeps working exactly as described above in every version through 9.1.\n\nA second close on an already-closed channelId is an error, which is why `catch {close $chan}` is the standard idiom for unconditional cleanup.",
            source: "Tcl close(n)",
            examples: "proc withOpenFile {filename channelVar script} {\n    upvar 1 $channelVar chan\n    set chan [open $filename]\n    catch {uplevel 1 $script} result options\n    close $chan\n    return -options $options $result\n}\n\n# Tcl 8.6+: half-close the write side of a pipe once the request is sent,\n# then read the rest of the reply before finishing the close.\nset p [open |[list somecmd] r+]\nputs $p $request\nclose $p write\nset reply [read $p]\nclose $p",
            return_value: "Empty string on success.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
