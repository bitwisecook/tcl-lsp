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

//! `tcl::process` — subprocess management ensemble (Tcl 9.0+, TIP 462).
//!
//! Absent from Tcl 8.4, 8.5, and 8.6 — no `process.html`/`process.htm`
//! manpage in any 8.x `TclCmd/` tree, and no `process` entry in the 8.6
//! command index. Introduced in Tcl 9.0; the 9.0.4 and 9.1b0
//! `generic/tclProcess.c` sources are identical apart from the 9.1
//! `Tcl_Size`/`Tcl_ObjCmdProc2` ABI-widening refactor — same four
//! subcommands, same `-wait`/`--` switches, same defaults, same
//! status-dict shape in both releases.
//!
//! Pure-host capability — never reachable in WASM/WASI builds (no
//! process model), so the registration exists for LSP recognition
//! and the runtime drops it on the trapping-stub list.  Two name
//! forms are registered for the namespace-relative and fully-
//! qualified spellings.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tcl::process subcommand ?arg ...?",
    dialects: None,
}];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    // `Process`, not `Unknown`: every subcommand below touches Tcl's
    // subprocess-bookkeeping tables (or the `autopurge` flag guarding
    // them), never a Tcl variable or generic interpreter state — see
    // each `SubCommand`'s own, more precise `side_effects` entry below,
    // which `dialect_side_effect_hints`
    // (`tcl-compiler/src/side_effects.rs`) prefers over this
    // command-level fallback whenever the subcommand is known.
    target: SideEffectTarget::Process,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        // `TCL90_PLUS` alone already excludes every modelled non-core
        // dialect: irules/iapps/tmsh/the EDA vendor shells all cap
        // below Tcl 9.0 (`tcl-dialect/src/profile.rs`'s per-profile
        // `version_ceiling`), and expect/tk/itcl have no
        // `process`-named registration of their own (grepped each
        // `commands/<dialect>/` directory — no matches). `bpf` is the
        // one profile whose `availability_mask` does reach Tcl 9.0
        // (`DialectSet::TCL90.union(DialectSet::BPF)` — a genuine
        // embedded Tcl 9.0 core), so this spec's `TCL90_PLUS` gate
        // intersects it and `tcl::process` resolves there today — just as
        // the sandbox-banned `exec`/`open`/`socket` (each now `ALL_TCL`,
        // whose Tcl-9.0 bit that same mask intersects) also resolve there,
        // even though
        // eBPF's kernel-verified execution model has no process/fork/
        // exec facility of its own. That gap predates this spec and
        // spans more than one command, so it is left alone here rather
        // than narrowed unilaterally for `tcl::process` alone.
        dialects: Some(DialectSet::TCL90_PLUS),
        // BYTE_COMPILED follows this codebase's convention (see
        // `traits.rs`'s doc comment): "recognised core builtin", not
        // literal bytecode compilation of the ensemble head itself. Its
        // four subcommands do each carry a real `CompileProc`
        // (`TclCompileBasic0ArgCmd` / `TclCompileBasicMin0ArgCmd` /
        // `TclCompileBasic0Or1ArgCmd`, `tclProcessImplMap` in
        // `tclProcess.c`), which only reinforces the classification.
        //
        // `Traits::SAFE_INTERP_HIDDEN` does NOT apply, despite every
        // subcommand being blocked in a safe interpreter: unlike the
        // commands that trait documents (a whole-command
        // `Tcl_HideCommand`, "invalid command name" on any call), the
        // `tcl::process` / `::tcl::process` ensemble word itself is
        // never hidden. Tcl 9.0.4's `unsafeEnsembleCommands[]`
        // (`generic/tclBasic.c`) lists all four subcommands
        // individually (`{"process", "list"}`, `"status"`, `"purge"`,
        // `"autopurge"`, annotated "has ONLY unsafe commands!") but
        // carries no `{"process", NULL}` whole-command entry; Tcl
        // 9.1b0's refactored equivalent (`tclProcessImplMap[]`'s
        // per-entry `unsafe` field, consumed by the rewritten
        // `TclHideUnsafeCommands`) hides the identical four. Either way
        // only the subcommand dispatch targets are replaced with a stub
        // that unconditionally raises "not allowed to invoke subcommand
        // ... of process" — the ensemble word keeps resolving. See the
        // hover snippet below.
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Manage subprocesses created by exec or a command-pipeline open, tracked by process id.",
            synopsis: &[
                "tcl::process autopurge ?flag?",
                "tcl::process list",
                "tcl::process purge ?pids?",
                "tcl::process status ?switches? ?pids?",
            ],
            snippet: "Tracks the subprocesses started by exec and by the |command form of open, referencing each by its process identifier (PID); it starts nothing itself. autopurge controls whether a subprocess's bookkeeping is discarded automatically as soon as it exits — right after exec returns or an open pipe channel closes — and defaults to on; switch it off to keep a terminated process's status available for later inspection, and reclaim it explicitly with purge. list reports every PID Tcl is still tracking, whether running or terminated but not yet purged. purge discards bookkeeping for terminated subprocesses — every one it knows about, or only those named in pids — silently skipping a pid that is still running or unrecognised; it also reaps any freshly-exited children first, so it can clean up processes that only just terminated. status returns a dict keyed by PID: a still-running process maps to the empty string, and a terminated one maps to a list {code ?msg errorCode?}, where code is its completion code (0 for a normal exit) and msg/errorCode appear only when it exited abnormally, was killed, or was suspended by a signal. By default status polls once and returns immediately; -wait blocks until a status is available — for every PID in pids, or for every tracked subprocess if pids is omitted. Querying a terminated process's status through this subcommand purges its bookkeeping afterwards, provided autopurge is still on. Both the tcl::process and ::tcl::process spellings resolve to this command. Inside a safe interpreter the command itself still resolves, but every subcommand is blocked: each call raises \"not allowed to invoke subcommand ... of process\".",
            source: "Tcl process(n)",
            examples: "set pids [exec sleep 30 &]\nputs \"tracked: [tcl::process list]\"\nset status [tcl::process status -wait $pids]\nputs $status\n\ntcl::process autopurge 0\nset pids [exec false &]\ntcl::process status -wait $pids\ntcl::process purge $pids",
            return_value: "Depends on the subcommand: autopurge returns the (possibly just-set) boolean flag; list returns a list of PIDs; purge returns the empty string; status returns a dict mapping each queried PID to its status — the empty string for a still-running process, or a list {code ?msg errorCode?} for one that has exited, where msg and errorCode are present only when code is nonzero.",
        }),
        ..CommandSpec::DEFAULT
    }
}

// `-wait`'s switch-scanning loop (`ProcessStatusObjCmd`,
// `generic/tclProcess.c`, identical in the 9.0.4 and 9.1b0 sources)
// re-enters on another `-wait` without breaking, so it may legally
// repeat (`status -wait -wait pids`); only a non-switch word or `--`
// stops switch scanning, and `--` itself can appear at most once,
// after any `-wait`s and before the single trailing `pids` argument.
// That is why `status`'s own `arity` below has no upper bound.
static STATUS_OPTIONS: [OptionSpec; 2] = [
    OptionSpec {
        name: "-wait",
        value: OptionValue::flag(),
        detail: "Block until status is available for pids (or, if pids is omitted, for every tracked subprocess) instead of polling once and returning immediately.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "Marks the end of switches; the following argument is treated as pids even if it starts with -.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

static SUBCOMMANDS: [SubCommand; 4] = [
    SubCommand {
        name: "autopurge",
        arity: Arity::new(0, 1),
        detail: "Query the autopurge flag, or set it first when flag (a boolean) is given; returns the resulting value either way. Defaults to on.",
        synopsis: "tcl::process autopurge ?flag?",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Process,
            // Always reads back the (possibly just-set) flag; writes it
            // only when `flag` is given — the static spec can't see
            // whether a call passes one, so declare the union.
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::new(0, 0),
        detail: "Return the PIDs of every tracked subprocess: still running, or terminated but not yet purged.",
        synopsis: "tcl::process list",
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Process,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "purge",
        arity: Arity::new(0, 1),
        detail: "Discard bookkeeping for terminated subprocesses, optionally limited to pids. Silently ignores a pid that is still running or unrecognised.",
        synopsis: "tcl::process purge ?pids?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Process,
            // Reaps any freshly-exited children unconditionally
            // (`Tcl_ReapDetachedProcs`) before reading the table to
            // decide what to delete.
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "status",
        // Unbounded, not 3 — see the comment on `STATUS_OPTIONS` above:
        // `-wait` may repeat, so there is no fixed word-count ceiling.
        arity: Arity::at_least(0),
        detail: "Return a dict mapping each tracked PID to its status: empty while still running, or a list once terminated. -wait blocks for a status instead of polling once.",
        synopsis: "tcl::process status ?switches? ?pids?",
        return_type: Some(TclType::Dict),
        options: &STATUS_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Process,
            // Refreshing a process's status can call Tcl_WaitPid
            // (reaping it at the OS level) and, when autopurge is on,
            // delete its bookkeeping entry right after reporting it —
            // so a plain `status` query can both reap and purge, not
            // just read.
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the namespace-relative form `tcl::process`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::process")
}

/// Command spec for the fully-qualified form `::tcl::process`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::process")
}
