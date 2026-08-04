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
    // Some subcommands only read (`exists`, `issafe`, `aliases`, `hidden`,
    // `target`, a bare `limit`/`recursionlimit`/`debug` query, ...) and
    // others only write (`create`, `delete`, the `alias` create form,
    // `hide`, `expose`, `marktrusted`, `share`, `transfer`, a `limit`/
    // `cancel`/`debug`/`recursionlimit` set form, ...); this command-level
    // default declares the union across every subcommand, the same way
    // `open`'s FileIo effect declares both for its access-mode-dependent
    // read/write mix.
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "interp subcommand ?arg arg ...?",
    dialects: None,
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

/// `limit`'s `limitType` (index 1, after `path`) takes one of these two
/// keywords — always a single bare word, never a list, so the index is
/// safe to close (contrast `open`'s access argument, which can be a
/// multi-flag list and so stays completion-only via `arg_values` alone).
///
/// The canonical spelling is `commands` (plural), not the manpage prose's
/// `command` (singular, "Command limits (of type command)" in the
/// RESOURCE LIMITS section of every one of tcl8.5/8.6/9.0/9.1's
/// TclCmd/interp.html — the doc text was never corrected). The real,
/// version-independent keyword is `commands`: `tclInterp.c`'s
/// `static const char *const limitTypes[] = { "commands", "time", NULL };`
/// is identical across the core-8-5-19, core-8-6-14, core-9-0-4, and
/// current `main` (9.1) source trees, and a live tclsh 8.6.14's own
/// error text confirms it (`bad limit type "xyz": must be commands or
/// time`). `command` (singular) still resolves — but only because it
/// happens to be a unique *prefix* of `commands`, the same
/// `arg_values_accept_prefix` path that also accepts `comm`/`c` below —
/// so listing the singular spelling as the canonical closed value was
/// backwards: it accepted the abbreviation while rejecting the literal
/// canonical word itself (`commands` is not a prefix of `command`).
const LIMIT_TYPE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "commands",
        detail: "Restrict the total number of Tcl commands the interpreter may execute.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "time",
        detail: "Restrict how long the interpreter may run, as an absolute time.",
        ..ArgValue::DEFAULT
    },
];

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
        // Positional arity is `?path? ?result?` only (0..=2); the
        // `-unwind`/`--` option words are consumed by the leading-option
        // skip, not counted here (same convention as `create` below). A
        // prior `Arity::at_least(0)` accepted an unbounded tail and masked
        // a genuine extra-argument error — confirmed against tclsh 8.6.14:
        // a path plus two more words raises a "wrong # args: should be
        // interp cancel ?-unwind? ?--? ?path? ?result?" error, and a bare
        // `interp cancel` (0 words) is valid (it cancels the invoking
        // interpreter's own evaluation).
        arity: Arity::new(0, 2),
        detail: "Cancel a script evaluation.",
        synopsis: "interp cancel ?-unwind? ?--? ?path? ?result?",
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-unwind",
                    value: OptionValue::flag(),
                    detail: "Unwind the evaluation stack without regard to any intervening catch, rather than stopping at the first enclosing one.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; needed when path itself looks like a switch (e.g. -safe).",
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
        analyser_hook: Some(crate::hooks::AnalyserHookId::InterpCreate),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "debug",
        // Added in Tcl 8.5, not 8.6: the tcl8.5/TclCmd/interp.html SYNOPSIS
        // and body both already document `interp debug path ?-frame
        // ?bool??`, and it is absent from the 8.4 SYNOPSIS.
        dialects: Some(DialectSet::TCL85_PLUS),
        // Positional arity is `path` plus the inline `-frame ?bool?` pair —
        // 1 to 3 raw words. `-frame` sits after the required `path`, so
        // (unlike `create`'s leading `-safe`/`--`) it is never stripped by
        // the leading-option skip and must be counted directly. Confirmed
        // against tclsh 8.6.14: `path`, `path -frame`, and `path -frame
        // $bool` all succeed, while a fourth trailing word raises a
        // "wrong # args: should be interp debug path ?-frame ?bool??" error.
        arity: Arity::new(1, 3),
        detail: "Get or set debug mode.",
        synopsis: "interp debug path ?-frame ?bool??",
        return_type: Some(TclType::String),
        options: const {
            &[OptionSpec {
                name: "-frame",
                value: OptionValue::value("boolean"),
                detail: "Enable exact per-command file/line tracking for `info frame` in the target interpreter (slower execution). Given with no value, only reports the current setting. Once turned on, cannot be turned back off.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
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
        analyser_hook: Some(crate::hooks::AnalyserHookId::InterpDelete),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::at_least(2),
        detail: "Evaluate script in another interpreter.",
        synopsis: "interp eval path arg ?arg ...?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::Body)],
        // The script runs in a *child* interpreter — a separate command /
        // variable space — so the analyser opens an isolated scope for it
        // rather than merging the child's definitions into the parent (a
        // parent `rename foo` must not edit a child `proc foo`).
        analyser_hook: Some(crate::hooks::AnalyserHookId::InterpEval),
        // Runs an arbitrary script — dynamic-dispatch consumers (memory-SSA
        // clobber classification, side-effect analysis) key off this.  The
        // trailing words concatenate into the script (`Tcl_ConcatObj`), so
        // the eval-family trait applies too.
        traits: Traits::EVALUATES_CODE.union(Traits::SCRIPT_CONCATENATES_ARGS),
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
        analyser_hook: Some(crate::hooks::AnalyserHookId::InterpExpose),
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
        analyser_hook: Some(crate::hooks::AnalyserHookId::InterpHide),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "invokehidden",
        // Positional arity is `path hiddenCmdName` plus an unbounded
        // `?arg ...?` tail; `-namespace`/`-global`/`--` sit after `path`
        // (not leading), but since the tail is already open-ended their
        // presence never changes the bound, so no arity fix is needed here
        // the way `debug`/`limit`/`cancel` needed one.
        arity: Arity::at_least(2),
        detail: "Invoke a hidden command.",
        synopsis: "interp invokehidden path ?-option ...? hiddenCmdName ?arg ...?",
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-global",
                    value: OptionValue::flag(),
                    detail: "Invoke the hidden command at the global level in the target interpreter, instead of the current call frame.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-namespace",
                    value: OptionValue::name("ns"),
                    // 8.4's `interp invokehidden path ?-global? hiddenCmdName
                    // ?arg ...?` only ever took `-global`; `-namespace` and
                    // `--` (below) first appear in the 8.5 SYNOPSIS's
                    // `?-option ...?` rewrite (confirmed by direct fetch of
                    // both tcl8.4/TclCmd/interp.html and
                    // tcl8.5/TclCmd/interp.html).
                    dialects: Some(DialectSet::TCL85_PLUS),
                    detail: "Namespace in which to invoke the hidden command. Ignored if -global is also given.",
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    dialects: Some(DialectSet::TCL85_PLUS),
                    detail: "Marks the end of switches, so hiddenCmdName may itself start with -.",
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
        // Positional words after `path limitType` come in `-option value`
        // pairs, with one exception: a lone trailing `-option` (querying
        // just that option) is also valid. Valid total word counts are
        // therefore 2 (bare query), 3 (single-option query), then the even
        // progression 4, 6, 8, ... (one or more full pairs) — never 5, 7,
        // 9, .... Confirmed against tclsh 8.6.14: 2, 3, 4, and 6 all
        // succeed, while 5 raises a "wrong # args" error.
        arity: Arity::stepped(2, Arity::UNLIMITED, 2).with_also_exact(3),
        detail: "Get or set resource limits.",
        synopsis: "interp limit path limitType ?-option? ?value ...?",
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-command",
                    value: OptionValue::script(),
                    detail: "Script run in the global namespace of the interpreter that set this option, invoked when the limited interpreter's limit is exceeded; may extend the limit to let evaluation continue. Common to both limit types.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-granularity",
                    value: OptionValue::value("n"),
                    detail: "How often, relative to the interpreter's consistent-state checkpoints, the limit is actually checked. Common to both limit types.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-milliseconds",
                    value: OptionValue::value("ms"),
                    detail: "Millisecond offset applied after -seconds. Only meaningful for the time limit type, given alongside -seconds.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-seconds",
                    value: OptionValue::value("epochSeconds"),
                    detail: "Epoch seconds (as from clock seconds) at which the time limit fires. Empty string clears the time limit. Only meaningful for the time limit type.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-value",
                    value: OptionValue::value("count"),
                    detail: "Number of commands the interpreter may execute before the command limit fires. Empty string clears the command limit. Only meaningful for the command limit type.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        arg_values: &[(1, LIMIT_TYPE_VALUES)],
        closed_value_args: &[1],
        // Confirmed against tclsh 8.6.14: `interp limit $i comm` and even
        // `interp limit $i c` both resolve as `commands` (same unique-prefix
        // ensemble dispatch as `chan close`'s direction argument).
        arg_values_accept_prefix: true,
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
        name: "set",
        // Brand new in Tcl 9.1: absent from the 9.0.4 SYNOPSIS and body,
        // present in the 9.1b0 SYNOPSIS and body as `interp set path
        // varName ?value?` / `child set varName ?value?`. Also confirmed
        // absent from Tcl 8.6.14: `interp set $child foo bar` raises
        // `bad option "set": must be alias, aliases, bgerror, cancel,
        // children, create, debug, delete, eval, exists, expose, hide,
        // hidden, issafe, invokehidden, limit, marktrusted,
        // recursionlimit, slaves, share, target, or transfer` — no `set`
        // in that list.
        dialects: Some(DialectSet::TCL91),
        arity: Arity::new(2, 3),
        detail: "Read or write a variable in another interpreter.",
        synopsis: "interp set path varName ?value?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
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
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::HAS_INTERP_EVAL
            | Traits::HAS_DESTRUCTIVE_OPS
            | Traits::LANGUAGE_KEYWORD
            | Traits::DYNAMIC_EVAL_BODY
            // `interp alias` / `hide` / `expose` / `invokehidden` all take
            // command identities as data (and an alias makes an external
            // name reach a script-defined proc), so command names are
            // observable wherever `interp` is used.
            | Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Create and manipulate Tcl interpreters.",
            synopsis: &["interp subcommand ?arg arg ...?"],
            snippet: "A child interpreter has its own namespace for commands, procedures, and variables, and connects back to its creator only through aliases (a command in the child that invokes a command in its parent or a sibling), shared environment variables, and resource-limit callbacks. `interp create -safe` creates a safe interpreter: dangerous commands such as exec, open, source, and socket are hidden rather than removed, so only a trusted ancestor can still reach them, via `interp invokehidden`. Tcl 8.4-8.6 called this relationship master/slave (`interp slaves`); Tcl 8.6 added the parent/child spelling alongside it (`interp children`), and Tcl 9.0 dropped the master/slave spelling entirely.",
            source: "Tcl interp(n)",
            examples: "set child [interp create -safe]\ninterp alias $child log {} puts\ninterp eval $child {log \"hello from the child\"}\ninterp delete $child",
            return_value: "Varies by subcommand: a boolean for exists/issafe, a list for aliases/hidden/children (or slaves), the new interpreter's name for create, or the evaluated script's result for eval; most state-changing subcommands (delete, hide, expose, marktrusted, share, transfer) return an empty string.",
        }),
        // `interp eval` / `interp invokehidden` run code in
        // another interpreter — cross-interp code injection (T105).
        taint_interp_eval_subcommands: &["eval", "invokehidden"],
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
