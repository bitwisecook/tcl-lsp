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

//! `pid` — return the process identifier of the current process, or of a
//! command pipeline's processes.
use crate::prelude::*;

/// `pid` reads process/channel-table state it never writes, but that
/// state lives outside the argument list — the OS process table for the
/// bare form, and the open-channel table's pipeline-pid bookkeeping for
/// `pid fileId` — the same reasoning that keeps `Traits::PURE` off
/// `close`/`eof`/`tell`/`fblocked`/`seek`, its FileIo-reading siblings
/// (see the `traits` field below). Both forms are modelled as a single
/// `FileIo` read, not `SideEffectTarget::Process`: in this registry
/// `Process` is reserved for commands that actually spawn or manage a
/// subprocess (`exec`, and the command-pipeline forms of `open` and
/// `readFile`) — `close_.rs` documents a deliberate decision to *not*
/// add a `Process` entry even though closing a pipe channel genuinely
/// waits on the child process and raises on its abnormal exit, so that
/// plain-Tcl `close` stays classified as exactly one `FileIo` effect
/// (`registry_commands::close_side_effects_differ_by_dialect`,
/// `side_effects_binding::classify_close_in_tcl_is_file_io`). `pid`
/// spawns nothing in either form and has an even weaker claim to
/// `Process` than that rejected `close` case: the bare form is a
/// same-process `getpid()`-style self-query, and `pid fileId` only
/// rereads Tcl's own already-cached channel-table bookkeeping recorded
/// when the pipeline was opened, never a fresh OS process query.
/// `pwd` — structurally the closest sibling, a niladic command reading a
/// per-process OS-level attribute that lives outside the argument list,
/// and grouped with `pid` in `pwd.rs`'s own reasoning — resolves this
/// identical FileIo-vs-Process judgement call to `FileIo` alone; `pid
/// fileId`'s explicit channel argument makes `FileIo` doubly
/// appropriate here, mirroring how `close`/`eof`/`tell` classify their
/// own channel argument. Read-only either way: unlike `open`'s pipe
/// form, `pid` never starts, stops, or otherwise mutates a process or a
/// channel.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "pid ?fileId?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `pid`.
///
/// `pid` is a plain core builtin, textually identical across Tcl 8.4
/// through 9.1: the `pid ?fileId?` synopsis, the DESCRIPTION (the
/// current process's identifier with no argument; the target
/// pipeline's process identifiers — or an empty list, for a channel
/// that is not a pipeline — with one), the SEE ALSO (`exec`, `open`),
/// and the KEYWORDS read word-for-word the same on all five official
/// manpages (8.4, 8.5, 8.6, 9.0, 9.1). Unlike its FileIo-only siblings
/// (`close`, `eof`, `tell`, `fblocked`, `seek`), the 9.0/9.1 manpages
/// never shrink to a "superceded by chan ..." pointer: there is no
/// `chan pid` subcommand in any version (`chan`'s own ensemble tops
/// out at `blocked`/`close`/…/`truncate` — see
/// `commands/tcl/chan_.rs`), so `pid` stays the only spelling
/// throughout.
///
/// `dialects: Some(DialectSet::ALL_TCL)` here is deliberate, not an
/// oversight: F5 iRules is the one modelled dialect that drops `pid` —
/// its TMM sandbox has no real process model, the same reasoning that
/// drops `exec`/`open`/`socket`/the rest of the filesystem-and-process
/// surface. That `ALL_TCL` group carries no `IRULES` bit, so it never
/// intersects the bare `IRULES` availability mask and `pid` simply falls
/// out by plain intersection — there is no disable list. Expect layers a
/// real OS process model on a
/// real Tcl core and leaves plain `pid` untouched; it adds its own,
/// separate `exp_pid` command (`commands/expect/exp_pid.rs`) for the
/// *spawned child's* pid rather than replacing this one. No other
/// modelled dialect (iApps, tmsh, the EDA vendor shells, Tk, incr Tcl)
/// disables or alters `pid`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pid",
        dialects: Some(DialectSet::ALL_TCL),
        // Reads process/channel-table state that lives outside the
        // argument list rather than being a pure function of its own
        // arguments (see the `SIDE_EFFECTS` doc comment above) —
        // `close`/`eof`/`tell`/`fblocked` stay off `Traits::PURE` for
        // the identical reason.
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(0, 1),
        arg_roles: &[(0, ArgRole::Channel)],
        // Documented return is the current process's identifier (an
        // int) for the bare form. `pid fileId` instead yields the
        // pipeline's decimal-string process identifiers — a list, except
        // on a channel that is not a pipeline, where tclsh answers a pure
        // string. The hook below is that per-form refinement, the same
        // split already documented on `scan` (`commands/tcl/scan_.rs`).
        return_type: Some(TclType::Int),
        // One positional (`fileId`) is the pipeline form, whose result is the
        // pipeline's decimal-string pids rather than this process's id — and
        // not a guaranteed list either, so it types as unknown.
        return_type_hook: Some(ReturnTypeHookId::Pid),
        hover: Some(HoverSnippet {
            summary: "Return the process identifier of the current process, or of a command pipeline's processes.",
            synopsis: &["pid ?fileId?"],
            snippet: "Without fileId, returns the identifier of the current process as a single decimal string. With fileId — a channel identifier for a command pipeline opened with open |command — returns a list of the decimal-string process identifiers of every process in that pipeline, in order. If fileId names an open channel that is not a process pipeline, such as a plain file, the result is an empty list; fileId must otherwise name a currently open channel.",
            source: "Tcl pid(n)",
            examples: "set pipeline [open \"| zcat somefile.gz | grep foobar | sort -u\"]\nexec ps -fp [pid $pipeline] >@stdout\nputs [read $pipeline]\nclose $pipeline\n\nputs \"this interpreter is running as pid [pid]\"",
            return_value: "The current process's identifier as a decimal string, or, given fileId, a list of the decimal-string process identifiers of every process in that pipeline — empty if fileId is an open channel that is not a process pipeline.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
