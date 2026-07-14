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

//! `interp` — create and manipulate Tcl interpreters.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "interp subcommand ?arg arg ...?",
}];

/// `interp alias srcPath srcCmd targetPath targetCmd ?arg ...?` (create form) —
/// `targetCmd` (index 3, after the `alias` subcommand word) is a command prefix
/// invoked with the aliased command's runtime args appended to any baked
/// `?arg...?`, so the count is variadic (`Unknown`, referenced but not
/// arity-checked). The 2-arg query form (`interp alias srcPath srcCmd`) has no
/// target.
fn interp_alias_command_prefixes(args: &[&str]) -> Vec<(u8, AppendedArity)> {
    if args.len() >= 4 {
        vec![(3, AppendedArity::Unknown)]
    } else {
        Vec::new()
    }
}

/// `interp bgerror path ?cmdPrefix?` — the optional background-error handler
/// (index 1, after `bgerror`) is a command prefix invoked with the error
/// message + return options (variadic ⇒ `Unknown`).
fn interp_bgerror_command_prefixes(args: &[&str]) -> Vec<(u8, AppendedArity)> {
    if args.len() >= 2 {
        vec![(1, AppendedArity::Unknown)]
    } else {
        Vec::new()
    }
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "alias",
        arity: Arity::at_least(2),
        detail: "Manage command aliases.",
        synopsis: "interp alias path cmd",
        return_type: Some(TclType::List),
        command_prefix_resolver: Some(interp_alias_command_prefixes),
        analyser_hook: Some(crate::hooks::AnalyserHookId::InterpAlias),
        command_table_effect: Some(crate::command_table::CommandTableEffect::CreatesAliases),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "aliases",
        arity: Arity::new(0, 1),
        detail: "List aliases.",
        synopsis: "interp aliases ?path?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bgerror",
        // Added in Tcl 8.5 (TIP 221).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::new(1, 2),
        detail: "Get or set background error handler.",
        synopsis: "interp bgerror path ?cmdPrefix?",
        return_type: Some(TclType::String),
        command_prefix_resolver: Some(interp_bgerror_command_prefixes),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cancel",
        // Added in Tcl 8.6 (TIP 285).
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(0),
        detail: "Cancel a script evaluation.",
        synopsis: "interp cancel ?-unwind? ?--? ?result?",
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-unwind",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        // Positional arity is `?name?` only (0..=1); the `-safe` / `--`
        // option words are consumed by the leading-option skip, not counted
        // here. A prior `Arity::new(0, 2)` masked a genuine extra-name error.
        arity: Arity::new(0, 1),
        detail: "Create a child interpreter.",
        synopsis: "interp create ?-safe? ?--? ?name?",
        // `interp create NAME` binds NAME as the child interpreter's command
        // (`NAME eval {…}` dispatches on it).  Index 0 is after the `create`
        // word; a `-safe` / `--` flag there (or a missing, auto-generated
        // name) is skipped by the consumer.
        defines_command_at: Some(0),
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-safe",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "debug",
        // Added in Tcl 8.6 (TIP 378 — TIP 233 is an unrelated proposal about
        // Tcl_SetTimeProc/virtualised time).
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        detail: "Control debug mode.",
        synopsis: "interp debug path ?-frame ?bool??",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::at_least(0),
        detail: "Delete interpreters.",
        synopsis: "interp delete ?path ...?",
        // `Tcl_InterpObjCmd` (tclInterp.c, `OPT_DELETE` arm) tears the
        // child interpreter down (`Tcl_DeleteInterp`) and errors when the
        // path no longer names one — `catch {interp delete $child}` is the
        // documented fire-and-forget idiom the W302 suppression keys off.
        destructive: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::at_least(2),
        detail: "Evaluate script in another interpreter.",
        synopsis: "interp eval path arg ?arg ...?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::Body)],
        // Runs an arbitrary script — dynamic-dispatch consumers (memory-SSA
        // clobber classification, side-effect analysis) key off this.
        traits: Traits::EVALUATES_CODE,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        // `path` is optional in every release: `interp exists` with no path
        // returns 1 (the current interpreter always exists).
        arity: Arity::new(0, 1),
        detail: "Check if interpreter exists.",
        synopsis: "interp exists ?path?",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "expose",
        arity: Arity::new(2, 3),
        detail: "Expose a hidden command.",
        synopsis: "interp expose path hiddenCmdName ?exposedCmdName?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hidden",
        // `path` is optional (`interp hidden` lists the current interpreter's
        // hidden commands).
        arity: Arity::new(0, 1),
        detail: "List hidden commands.",
        synopsis: "interp hidden ?path?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hide",
        arity: Arity::new(2, 3),
        detail: "Hide a command.",
        synopsis: "interp hide path exposedCmdName ?hiddenCmdName?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "invokehidden",
        arity: Arity::at_least(2),
        detail: "Invoke a hidden command.",
        synopsis: "interp invokehidden path ?-option ...? hiddenCmdName ?arg ...?",
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-global",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-namespace",
                    value: OptionValue::name("ns"),
                    detail: "Namespace in which to invoke the hidden command.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "issafe",
        // `path` is optional (`interp issafe` reports on the current
        // interpreter).
        arity: Arity::new(0, 1),
        detail: "Check if interpreter is safe.",
        synopsis: "interp issafe ?path?",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "limit",
        // Added in Tcl 8.5 (TIP 143).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(2),
        detail: "Get or set resource limits.",
        synopsis: "interp limit path limitType ?-option value ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "marktrusted",
        arity: Arity::exact(1),
        detail: "Mark interpreter as trusted.",
        synopsis: "interp marktrusted path",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "recursionlimit",
        arity: Arity::new(1, 2),
        detail: "Get or set recursion limit.",
        synopsis: "interp recursionlimit path ?newlimit?",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "share",
        arity: Arity::exact(3),
        detail: "Share a channel.",
        synopsis: "interp share srcPath channelId destPath",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "target",
        arity: Arity::exact(2),
        detail: "Get alias target.",
        synopsis: "interp target path alias",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "transfer",
        arity: Arity::exact(3),
        detail: "Transfer a channel.",
        synopsis: "interp transfer srcPath channelId destPath",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        // Legacy name for `interp children`; removed in Tcl 9.0 (the
        // slave/master → child/parent rename), so 8.4-8.6 only.
        dialects: Some(DialectSet::TCL8X),
        arity: Arity::new(0, 1),
        detail: "Returns a Tcl list of the names of all the child interpreters.",
        synopsis: "interp slaves ?path?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "children",
        // Added in Tcl 8.6 (child/parent terminology; the preferred name for
        // the older `interp slaves`).
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(0, 1),
        detail: "Returns a Tcl list of the names of all the child interpreters associated with the interpreter identified by path.",
        synopsis: "interp children ?path?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "interp",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::HAS_INTERP_EVAL
            | Traits::HAS_DESTRUCTIVE_OPS
            | Traits::LANGUAGE_KEYWORD
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Create and manipulate Tcl interpreters",
            synopsis: &[
                "interp subcommand ?arg arg ...?",
                "interp subcommand ?arg ...?",
            ],
            snippet: "This command makes it possible to create one or more new Tcl interpreters that co-exist with the creating interpreter in the same application.",
            source: "Tcl man page interp.n",
            examples: "",
            return_value: "",
        }),
        // `interp eval` / `interp invokehidden` run code in
        // another interpreter — cross-interp code injection (T105).
        taint_interp_eval_subcommands: &["eval", "invokehidden"],
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
