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

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "trace option ?arg arg ...?",
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
    if args
        .first()
        .is_some_and(|w| resolve_trace_type(w) == Some("variable"))
        && args.len() >= 2
    {
        return vec![(1, ArgRole::VarWrite)];
    }
    Vec::new()
}

/// Same arg-role pattern for `trace remove variable` — keeps
/// registry consistency with `trace add variable` so consumers can
/// query both spellings via the same `ArgRole::VarWrite` lookup.
fn trace_remove_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args
        .first()
        .is_some_and(|w| resolve_trace_type(w) == Some("variable"))
        && args.len() >= 2
    {
        return vec![(1, ArgRole::VarWrite)];
    }
    Vec::new()
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
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::ESTABLISHES_VARIABLE_TRACE),
        arity: Arity::exact(4),
        detail: "Arrange for a command to be executed on the specified operation.",
        synopsis: "trace add type name ops commandPrefix",
        arg_role_resolver: Some(trace_add_arg_roles),
        command_prefix_resolver: Some(trace_add_command_prefixes),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(2),
        detail: "Return trace info for the given name.",
        synopsis: "trace info type name",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::ESTABLISHES_VARIABLE_TRACE),
        arity: Arity::exact(4),
        detail: "Remove a trace.",
        synopsis: "trace remove type name ops commandPrefix",
        arg_role_resolver: Some(trace_remove_arg_roles),
        command_prefix_resolver: Some(trace_remove_command_prefixes),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "variable",
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::ESTABLISHES_VARIABLE_TRACE),
        arity: Arity::exact(3),
        detail: "Arrange for command to be executed whenever variable name is accessed. Deprecated in favour of trace add variable.",
        synopsis: "trace variable name ops command",
        arg_role_resolver: Some(trace_legacy_arg_roles),
        command_prefix_resolver: Some(trace_variable_command_prefixes),
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vdelete",
        traits: Traits::TARGETS_VARIABLE_BY_NAME.union(Traits::ESTABLISHES_VARIABLE_TRACE),
        arity: Arity::exact(3),
        detail: "Delete a variable trace. Deprecated in favour of trace remove variable.",
        synopsis: "trace vdelete name ops command",
        arg_role_resolver: Some(trace_legacy_arg_roles),
        command_prefix_resolver: Some(trace_vdelete_command_prefixes),
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vinfo",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(1),
        detail: "Return trace information for the given variable. Deprecated in favour of trace info variable.",
        synopsis: "trace vinfo name",
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trace",
        traits: Traits::CREATES_BARRIER | Traits::CREATES_DYNAMIC_BARRIER,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Monitor variable accesses, command usages and command executions",
            synopsis: &["trace option ?arg arg ...?"],
            snippet: "Arranges for commands to be executed whenever certain operations are invoked.",
            source: "Tcl man page trace.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
