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

//! `memory` — control Tcl's built-in memory-debugging capabilities (debug
//! builds only).
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "memory option ?arg arg ...?",
    dialects: None,
}];

/// Command-level default side effect: most `memory` subcommands flip a
/// process-global flag in the memory-debugging subsystem rather than
/// touching the filesystem. The three subcommands that write a file
/// (`active`/`objs`/`onexit`) override this with their own `FileIo`
/// entry; `info` (a pure query) overrides it with an empty list.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Legal values for the `on`/`off` toggle subcommands (`init`, `trace`,
/// `validate`). All five fetched manpages (8.4-9.1) document the value as
/// the closed pair `[on|off]`, not the fuller boolean vocabulary
/// (`true`/`false`/`yes`/`no`/`1`/`0`) `Tcl_GetBoolean` accepts elsewhere
/// in Tcl, so the set is modelled as closed to exactly these two words.
const ON_OFF_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "on",
        detail: "Enable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "off",
        detail: "Disable.",
        ..ArgValue::DEFAULT
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "active",
        arity: Arity::exact(1),
        detail: "Write a list of every currently allocated memory block to file.",
        synopsis: "memory active file",
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "break_on_malloc",
        arity: Arity::exact(1),
        detail: "After count further allocations, print a message and send the process SIGINT in an attempt to enter a C debugger.",
        synopsis: "memory break_on_malloc count",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(0),
        detail: "Return a report of the total allocations and frees since Tcl started, the current packets and bytes allocated, and their historical maximums.",
        synopsis: "memory info",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "init",
        arity: Arity::exact(1),
        detail: "Turn preinitialization of newly allocated memory with bogus bytes on or off, to help catch use of uninitialized values.",
        synopsis: "memory init on|off",
        arg_values: &[(0, ON_OFF_VALUES)],
        closed_value_args: &[0],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "objs",
        arity: Arity::exact(1),
        detail: "Write every currently allocated Tcl_Obj value, together with its allocation site, to file -- useful for finding Tcl_Obj leaks.",
        synopsis: "memory objs file",
        // Added in Tcl 8.5: the 8.4 manpage's DESCRIPTION enumerates only
        // active/break_on_malloc/info/init/onexit/tag/trace/
        // trace_on_at_malloc/validate (nine subcommands, no `objs`); 8.5
        // through 9.1 all list ten, adding `objs` in the same position
        // (raw manpage HTML fetched and diffed for every version).
        dialects: Some(DialectSet::TCL85_PLUS),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "onexit",
        arity: Arity::exact(1),
        detail: "Write a list of all still-allocated memory to file while the memory subsystem finalizes, to help confirm memory is cleaned up at process exit.",
        synopsis: "memory onexit file",
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tag",
        arity: Arity::exact(1),
        detail: "Set the tag string attached to subsequent allocations; the tag is printed alongside each packet in memory active and memory onexit output.",
        synopsis: "memory tag string",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trace",
        arity: Arity::exact(1),
        detail: "Turn tracing of every allocation and free to stderr on or off.",
        synopsis: "memory trace on|off",
        arg_values: &[(0, ON_OFF_VALUES)],
        closed_value_args: &[0],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trace_on_at_malloc",
        arity: Arity::exact(1),
        detail: "Enable memory tracing starting after count allocations have occurred, to skip past startup noise before a suspected problem.",
        synopsis: "memory trace_on_at_malloc count",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "validate",
        arity: Arity::exact(1),
        detail: "Turn guard-zone validation of every allocated block on or off on every allocation and free; detects an overwrite immediately, at a significant performance cost.",
        synopsis: "memory validate on|off",
        arg_values: &[(0, ON_OFF_VALUES)],
        closed_value_args: &[0],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `memory`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "memory",
        // Universal core Tcl 8.4-9.1: present under `TclCmd` in every
        // fetched manpage, with an identical `memory option ?arg arg
        // ...?` synopsis throughout. Its real availability gate is a
        // *compile-time* flag (`TCL_MEM_DEBUG`, see the hover snippet),
        // not a Tcl version or dialect, and this registry has no axis for
        // "debug build only" — captured in prose instead.
        //
        // Excluded from f5-irules via the K36322151 subtractive
        // `IRULES_DISABLED_COMMANDS` list in `tcl-dialect/src/profile.rs`
        // (already lists `"memory"`), the same mechanism `cd`/`load`/
        // `open`/`unload` rely on rather than a `CommandSpec`-level
        // dialect restriction: `tcl-registry/tests/dialect_profile.rs`'s
        // `irules_disable_list_is_load_bearing` test requires the spec
        // stay reachable under the bare `IRULES` mask (i.e. keep
        // `dialects: None`, or at least not exclude `IRULES`) for that
        // list entry to remain load-bearing rather than dead weight.
        // No other modelled dialect (f5-iapps/f5-tmsh/Expect/the EDA
        // vendor shells/Tk) has a subtractive disable mechanism of its
        // own, and none plausibly ships a `TCL_MEM_DEBUG` build either,
        // but there is no evidence (and no available mechanism) to model
        // that here per-dialect, so this stays universal.
        dialects: None,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        forms: FORMS,
        hover: Some(HoverSnippet {
            summary: "Control Tcl's memory-debugging capabilities (debug builds only).",
            synopsis: &["memory option ?arg arg ...?"],
            snippet: "Available only when Tcl was compiled with memory debugging enabled (TCL_MEM_DEBUG defined at compile time) and after Tcl_InitMemory has run; the command does not exist at all in an ordinary release build. Backs the allocator's built-in leak- and overwrite-detection tooling (ckalloc/ckfree through Tcl 8.6, renamed Tcl_Alloc/Tcl_Free from Tcl 9.0 -- a documentation rename only, not a Tcl-level behaviour change), used mainly during Tcl's own core development. The objs subcommand was added in Tcl 8.5; every other subcommand is unchanged across 8.4-9.1.",
            source: "Tcl man page memory.n",
            examples: "memory tag start\nmemory trace on\nset data [someProc]\nmemory trace off\nputs [memory info]",
            return_value: "The empty string, except memory info, which returns a multi-line allocation-statistics report.",
        }),
        ..CommandSpec::DEFAULT
    }
}
