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

//! `trace` — monitor variable accesses, command usages and executions.
use crate::prelude::*;

// Command-level fallback for call sites where the actual subcommand can't
// be resolved statically (`dialect_side_effect_hints` in
// `tcl-compiler/src/side_effects.rs` prefers a resolved subcommand's own
// `side_effects` over this one, so this only fires as the conservative
// default). Declares the union of what any subcommand can do: `info`,
// `vinfo`, `remove`, and `vdelete` read the existing trace table, while
// `add`, `remove`, `variable`, and `vdelete` write it.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "trace option ?arg arg ...?",
    dialects: None,
}];

/// Resolve a `trace add|remove` type word (`variable`/`command`/
/// `execution`) the way C Tcl 9.0's `Tcl_GetIndexFromObj` does: a
/// unique, non-empty prefix is accepted, so `trace add v x read h` /
/// `trace add var x read h` install the same variable trace as the
/// full spelling (checked against tclsh 8.6.14).
fn resolve_trace_type(word: &str) -> Option<&'static str> {
    const TYPES: &[&str] = &["variable", "command", "execution"];
    if word.is_empty() {
        return None;
    }
    let mut hits = TYPES.iter().copied().filter(|t| t.starts_with(word));
    let first = hits.next()?;
    if hits.next().is_some() {
        return None; // ambiguous prefix
    }
    Some(first)
}

/// Arg-role resolver for `trace add`.
///
/// `trace add variable name ops commandPrefix` writes to `name` —
/// the trace handler can rewrite the variable at runtime, so SSA
/// must see `name` as a definition site.
///
/// The resolver only fires for the `variable` form (accepting any
/// unique-prefix abbreviation of it, e.g. `var`/`v`) so
/// `trace add execution` and `trace add command` (which take a
/// command name, not a variable) don't appear as SSA defs.
fn trace_add_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() < 2 {
        return Vec::new();
    }
    match args.first().and_then(|w| resolve_trace_type(w)) {
        // The traced variable can be rewritten by the handler, so SSA must see
        // `name` as a definition site.
        Some("variable") => vec![(1, ArgRole::VarWrite)],
        // `command` / `execution` trace a command by name — a reference
        // navigation follows to the named command (not invoked here; the
        // handler prefix that follows is a separate `CommandPrefix`).
        Some("command" | "execution") => vec![(1, ArgRole::CommandName)],
        _ => Vec::new(),
    }
}

/// Same arg-role pattern for `trace remove variable` — keeps
/// registry consistency with `trace add variable` so consumers can
/// query both spellings via the same `ArgRole::VarWrite` lookup.
fn trace_remove_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() < 2 {
        return Vec::new();
    }
    match args.first().and_then(|w| resolve_trace_type(w)) {
        Some("variable") => vec![(1, ArgRole::VarWrite)],
        Some("command" | "execution") => vec![(1, ArgRole::CommandName)],
        _ => Vec::new(),
    }
}

/// The invoked arity of a `trace add/remove <type> name ops cmdPrefix`
/// callback (C Tcl 9.0): a `variable` trace fires `cmdPrefix name1 name2 op`
/// (3), a `command` trace fires `cmdPrefix oldName newName op` (3), an
/// `execution` trace fires 2–4 args (`enter`/`leave`/`enterstep`/`leavestep`),
/// so `AtLeast(2)`.  A `remove` only references the handler (it is matched, not
/// invoked), so it carries `Unknown` — recorded as a reference, not
/// arity-checked.
fn trace_type_command_prefix(args: &[&str], installing: bool) -> Vec<(u8, AppendedArity)> {
    // args after the subcommand word: `type name ops cmdPrefix` (index 3).
    if args.len() <= 3 {
        return Vec::new();
    }
    let arity = if installing {
        match args.first().and_then(|w| resolve_trace_type(w)) {
            Some("variable" | "command") => AppendedArity::Exactly(3),
            Some("execution") => AppendedArity::AtLeast(2),
            _ => AppendedArity::Unknown,
        }
    } else {
        AppendedArity::Unknown
    };
    vec![(3, arity)]
}

fn trace_add_command_prefixes(args: &[&str]) -> Vec<(u8, AppendedArity)> {
    trace_type_command_prefix(args, true)
}

fn trace_remove_command_prefixes(args: &[&str]) -> Vec<(u8, AppendedArity)> {
    trace_type_command_prefix(args, false)
}

/// Deprecated `trace variable name ops command` / `trace vdelete …` — the
/// command prefix is the 3rd word (index 2).  `variable` installs a
/// variable trace (`command name1 name2 op` → 3 args); `vdelete` only
/// references the handler (`Unknown`).
fn trace_legacy_command_prefix(args: &[&str], installing: bool) -> Vec<(u8, AppendedArity)> {
    if args.len() <= 2 {
        return Vec::new();
    }
    let arity = if installing {
        AppendedArity::Exactly(3)
    } else {
        AppendedArity::Unknown
    };
    vec![(2, arity)]
}

fn trace_variable_command_prefixes(args: &[&str]) -> Vec<(u8, AppendedArity)> {
    trace_legacy_command_prefix(args, true)
}

fn trace_vdelete_command_prefixes(args: &[&str]) -> Vec<(u8, AppendedArity)> {
    trace_legacy_command_prefix(args, false)
}

/// The `type` word taken by `trace add|remove|info` (relative index 0,
/// right after the subcommand word) — `command`, `execution`, or
/// `variable`, or any prefix of one of those that is unique among the
/// three (`Tcl_GetIndexFromObj`-style abbreviation; see
/// [`resolve_trace_type`], which implements the identical matching rule
/// for the arg-role/command-prefix resolvers above). Spelled identically
/// in the `trace add type name ops ?args?` synopsis line of every Tcl
/// release from 8.4 through 9.1 — the trace.n manpage is unchanged here
/// across the whole range. `type` is always exactly one word, never a
/// list, so (unlike [`TRACE_OPS_VALUES`] below) this set is exhaustive:
/// closed via `closed_value_args` with `arg_values_accept_prefix: true`.
const TRACE_TYPE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "command",
        detail: "Trace renames and deletions of a command.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "execution",
        detail: "Trace invocation of a command, and optionally every command nested inside it.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "variable",
        detail: "Trace reads, writes, unsets, and array operations on a variable.",
        ..ArgValue::DEFAULT
    },
];

/// The `ops`/`opList` value taken by `trace add|remove` (relative index
/// 2) — a Tcl LIST of one or more of the ten words below, the legal set
/// depending on which `type` (position 0) the call names: `command` type
/// takes `rename`/`delete`; `execution` type takes
/// `enter`/`leave`/`enterstep`/`leavestep`; `variable` type takes
/// `array`/`read`/`write`/`unset`. All ten words are spelled identically
/// in every Tcl release from 8.4 through 9.1. Because this position is a
/// *list* (`trace add variable x {read write} cb` is one argument
/// carrying two ops words), it is completion/hover data only — never
/// `closed_value_args` (see the pitfall noted on `open`'s `ACCESS_VALUES`
/// for the same reasoning: a value here can legitimately combine several
/// of these words, which whole-word `closed_value_args` matching cannot
/// express).
const TRACE_OPS_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "array",
        detail: "variable type: fires when the variable is accessed via the array command, while it is not (yet) a scalar.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "read",
        detail: "variable type: fires whenever the variable is read.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "write",
        detail: "variable type: fires whenever the variable is written.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "unset",
        detail: "variable type: fires whenever the variable is unset, explicitly or via procedure return.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "rename",
        detail: "command type: fires when the traced command is renamed to a non-empty name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "delete",
        detail: "command type: fires when the traced command is deleted (including a rename to the empty string).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "enter",
        detail: "execution type: fires just before the traced command runs.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "leave",
        detail: "execution type: fires just after the traced command runs.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "enterstep",
        detail: "execution type: fires just before every command executed inside the traced procedure.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "leavestep",
        detail: "execution type: fires just after every command executed inside the traced procedure.",
        ..ArgValue::DEFAULT
    },
];

/// The `ops` value taken by the deprecated `trace variable`/`trace
/// vdelete` (relative index 1) — NOT a Tcl list like the modern
/// [`TRACE_OPS_VALUES`], but per the trace.n manpage (unchanged across
/// 8.4-8.6, the only versions where this form exists) "a string
/// concatenation of the operations" with no separator, e.g. `rwu` for
/// read+write+unset: `array`/`read`/`write`/`unset` are abbreviated
/// `a`/`r`/`w`/`u` respectively. Combinable the same way `ops` itself is,
/// so likewise never `closed_value_args`.
const TRACE_LEGACY_OPS_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "a",
        detail: "array: fires when the variable is accessed via the array command.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "r",
        detail: "read: fires whenever the variable is read.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "w",
        detail: "write: fires whenever the variable is written.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "u",
        detail: "unset: fires whenever the variable is unset.",
        ..ArgValue::DEFAULT
    },
];

/// Arg-role resolver for the deprecated `trace variable name ops
/// command` / `trace vdelete name ops command` legacy forms — the
/// variable name is the word immediately after the subcommand
/// (relative index 0), mirroring [`trace_add_arg_roles`] for the
/// modern `trace add variable` spelling so SSA sees the same
/// definition-site behaviour regardless of which form the source
/// uses.
fn trace_legacy_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.is_empty() {
        return Vec::new();
    }
    vec![(0, ArgRole::VarWrite)]
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        // `trace add command|execution NAME …` targets a command by its
        // spelled name, so command identities are observable data here too.
        traits: Traits::TARGETS_VARIABLE_BY_NAME
            .union(Traits::ESTABLISHES_VARIABLE_TRACE)
            .union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(4),
        detail: "Arrange for commandPrefix to be invoked, with operation-specific arguments appended, whenever the variable/command/execution named by name undergoes one of the operations in ops. For type command or execution, name must already exist or this throws an error; for type variable, a nonexistent name is instead silently created without a value (visible to a namespace which query but not to info exists, until something writes it). Present, with this exact 4-argument shape, in every Tcl release from 8.4 through 9.1.",
        synopsis: "trace add type name ops commandPrefix",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_add_arg_roles),
        command_prefix_resolver: Some(trace_add_command_prefixes),
        arg_values: &[(0, TRACE_TYPE_VALUES), (2, TRACE_OPS_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        // `trace info command|execution NAME` reads traces off a command
        // named as data.
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(2),
        detail: "Return a list of the traces currently set on the command or variable name, one element per trace, each itself a two-element {opList commandPrefix} list, or an empty list if none are set. For type command or execution, a nonexistent name throws an error; for type variable, a nonexistent name likewise just yields an empty list.",
        synopsis: "trace info type name",
        pure: true,
        return_type: Some(TclType::List),
        arg_values: &[(0, TRACE_TYPE_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        // `trace remove command|execution NAME …` targets a command by its
        // spelled name.
        traits: Traits::TARGETS_VARIABLE_BY_NAME
            .union(Traits::ESTABLISHES_VARIABLE_TRACE)
            .union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(4),
        detail: "Remove a previously added trace matching type, name, ops, and commandPrefix exactly. For a variable name that does not exist, or a variable/command/execution trace that does not match, this is silently a no-op; for a nonexistent command or execution name it instead throws an error.",
        synopsis: "trace remove type name opList commandPrefix",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_remove_arg_roles),
        command_prefix_resolver: Some(trace_remove_command_prefixes),
        arg_values: &[(0, TRACE_TYPE_VALUES), (2, TRACE_OPS_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "variable",
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::ESTABLISHES_VARIABLE_TRACE),
        arity: Arity::exact(3),
        detail: "Arrange for command to be executed whenever variable name is accessed, using the legacy single-letter ops encoding (a/r/w/u, concatenated with no separator, e.g. rwu). Equivalent to trace add variable name ops command. Deprecated throughout 8.4-8.6; removed in Tcl 9.0 (absent from the 9.0/9.1 manpages' synopsis and body alike).",
        synopsis: "trace variable name ops command",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_legacy_arg_roles),
        command_prefix_resolver: Some(trace_variable_command_prefixes),
        arg_values: &[(1, TRACE_LEGACY_OPS_VALUES)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only) — the
        // 9.0/9.1 trace.n manpages drop both the SYNOPSIS line and the
        // entire "For backwards compatibility..." paragraph describing it.
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vdelete",
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::ESTABLISHES_VARIABLE_TRACE),
        arity: Arity::exact(3),
        detail: "Delete a variable trace added with trace variable or trace add variable, using the legacy single-letter ops encoding (a/r/w/u). Equivalent to trace remove variable name ops command. Deprecated throughout 8.4-8.6; removed in Tcl 9.0.",
        synopsis: "trace vdelete name ops command",
        return_type: Some(TclType::String),
        mutator: true,
        arg_role_resolver: Some(trace_legacy_arg_roles),
        command_prefix_resolver: Some(trace_vdelete_command_prefixes),
        arg_values: &[(1, TRACE_LEGACY_OPS_VALUES)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vinfo",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(1),
        detail: "Return trace information for the given variable, in the same {opList command} list shape as trace info variable. Equivalent to trace info variable name. Deprecated throughout 8.4-8.6; removed in Tcl 9.0.",
        synopsis: "trace vinfo name",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `trace`.
///
/// Per the trace.n manpage fetched for all five versions (8.4, 8.5, 8.6,
/// 9.0, 9.1): the ensemble shape (`add`/`remove`/`info`, each dispatching
/// on a `command`/`execution`/`variable` type) is already present, with
/// this exact wording bar `command` → `commandPrefix` terminology, in Tcl
/// 8.4 — it is not an 8.5+ addition. The only real version boundary is at
/// 9.0, which:
///   - Removes the three deprecated one-word-type subcommands (`trace
///     variable`/`trace vdelete`/`trace vinfo`) entirely — both their
///     SYNOPSIS lines and the whole "For backwards compatibility..."
///     paragraph documenting them are present in 8.4/8.5/8.6 and absent
///     from 9.0/9.1 (byte-identical omission in both). See the
///     `dialects: Some(DialectSet::TCL8X)` gate on `variable`/`vdelete`/
///     `vinfo` below.
///   - Sharpens (without contradicting) the `trace add variable`
///     callback's name1/name2 description: 9.0/9.1 spell out that name2
///     carries the array index even when name1 resolves to what looks
///     like a scalar (possible via `upvar` aliasing a single array
///     element) — folded into the hover snippet below as current,
///     version-neutral fact, since the 8.4-8.6 wording does not
///     contradict it, only under-specifies this one edge case.
///
/// 8.5 and 8.6 are otherwise identical to 8.4 in substance (terminology
/// and copy-editing only), and 9.1's trace.html is byte-for-byte
/// identical to 9.0's (bar the doc-anchor version banner) — no
/// 9.1-specific delta exists for this command.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trace",
        // Present and unrestricted: `trace` carries the `IRULES` bit
        // explicitly (`ALL_TCL.union(IRULES)`), so it resolves under the
        // bare `IRULES` mask, and every dialect that hosts a real Tcl core
        // (irules, iapps, tmsh, the EDA shells, expect, tk, itcl) carries
        // it unmodified. Its legal *subcommand* set still narrows per Tcl
        // version through each SubCommand's own `dialects` gate below. That
        // `TCL8X` gate on `variable`/`vdelete`/`vinfo` is a bare
        // Tcl-version mask, so per
        // `ProfileQueries::is_subcommand_available` it correctly extends to
        // every OTHER dialect whose `availability_mask` composes a real
        // embedded Tcl-version bit with its own vendor bit (f5-iapps and
        // f5-tmsh at `TCL85`, the EDA shells at `TCL85`/`TCL86`, Expect at
        // `TCL86`) — but NOT to f5-irules, whose mask is the bare `IRULES`
        // bit with no Tcl-version bit unioned in (iRules availability is
        // fully explicit per spec — a command carries the `IRULES` bit iff
        // iRules enables it, so the mask needs no Tcl-version bit). A pure
        // Tcl-version gate never intersects that bare mask regardless of
        // iRules' embedded-8.4 runtime — the same intersects-only
        // membership rule `tests/dialect_profile.rs`'s
        // `option_gating_honours_the_version_ceiling` documents for option
        // gating applies identically to subcommand gating — so whether real
        // F5 iRules exposes these three legacy forms is left unmodelled
        // here rather than guessed.
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        traits: Traits::CREATES_BARRIER | Traits::CREATES_DYNAMIC_BARRIER | Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Monitor variable accesses, command renames/deletions, and command executions by invoking a callback.",
            synopsis: &["trace option ?arg arg ...?"],
            snippet: "Dispatches on option to add, remove, info, or (deprecated, removed in Tcl 9.0) variable/vdelete/vinfo. trace add type name ops commandPrefix arranges for commandPrefix to be invoked, with operation-specific arguments appended, whenever the variable, command, or command execution named by name undergoes one of the operations in ops; for type command or execution, name must already exist or trace add throws an error, while for type variable a nonexistent name is instead silently created without a value. trace remove undoes a matching trace; trace info returns the traces currently set. type is variable, command, or execution (any unique abbreviation accepted). For a variable trace, ops is one or more of array/read/write/unset, and the callback receives commandPrefix name1 name2 op — name1 is the accessed variable's name and name2 the array index when the trace was set on an array or an array element (present even when name1 itself looks like a scalar, e.g. via an upvar alias to a single element); a read/write callback may itself rewrite the variable to override the traced operation's result, and returning an error from the callback aborts the read/write/access that triggered it; while a read or write callback runs, that same trace is temporarily disabled (so it cannot re-trigger itself), but an unset callback is the one exception — traces stay enabled during unset, so a handler that installs a new trace and touches the variable will see it fire. When several traces are set on the same variable they fire most-recently-added first, stopping at the first one that raises an error, and a trace on the array as a whole fires before one on a single element of it. For a command trace, ops is rename and/or delete, with the callback receiving commandPrefix oldName newName op; deletion cannot be prevented from inside the trace, since Tcl always removes the command once the callback returns, and a rename or delete performed from inside the callback does not re-trigger further traces of that same type. A command trace also never fires when its target disappears because the interpreter itself is being torn down, since there is then no interpreter left to run the callback in. For an execution trace, ops is one or more of enter/leave/enterstep/leavestep — enter/leave fire immediately before/after the traced command itself runs, enterstep/leavestep fire before/after every command nested inside it (meaningful only when name is a procedure) — with the callback receiving commandPrefix command-string op for enter/enterstep or commandPrefix command-string code result op for leave/leavestep; deleting the traced command from inside an enter/enterstep callback stops the pending execution, and while an execution callback runs, that same trace is likewise temporarily disabled. Multiple execution traces on the same name fire in reverse creation order for enter/enterstep and original creation order for leave/leavestep. trace add/remove/variable/vdelete return an empty string; trace info/vinfo return a list of the matching traces (an empty list if none are set), each element itself a two-element {opList commandPrefix} list.",
            source: "Tcl trace(n)",
            examples: "# Log every write to a configuration variable, and read the new value\n# back out via the name(s) the callback receives (not a closure over\n# the original variable name, since upvar can alias it under another).\nproc logWrite {name1 name2 op} {\n    upvar #0 $name1 var\n    puts \"$name1 -> $var\"\n}\ntrace add variable ::config(port) write logWrite\n\n# Trace every command entered while `compute` runs.\nproc traceStep {cmdString op} {\n    puts \"step: $cmdString\"\n}\ntrace add execution compute enterstep traceStep\n\n# Remove a trace again with the exact same arguments used to add it.\ntrace remove variable ::config(port) write logWrite",
            return_value: "Depends on the subcommand: an empty string (add, remove, and the deprecated variable/vdelete), or a list of the matching traces — one element per trace, each itself a two-element {opList commandPrefix} list (info, and the deprecated vinfo).",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
