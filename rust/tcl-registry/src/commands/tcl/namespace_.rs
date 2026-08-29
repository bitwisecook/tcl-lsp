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

//! `namespace` — create and manipulate contexts for commands and variables.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "namespace subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// `namespace upvar` changed its zero-pair arity at Tcl 8.6: 8.5 requires at
/// least one `otherVar myVar` pair, while 8.6+ permits the namespace alone as a
/// no-op. Keep the release split in registry form data so runtime and static
/// consumers ask the same source of truth.
const NAMESPACE_UPVAR_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "tcl8.5",
        arity: Arity::stepped(3, Arity::UNLIMITED, 2),
        surface: Some(SpecSurface::TCL85),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "tcl8.6+",
        arity: Arity::stepped(1, Arity::UNLIMITED, 2),
        surface: Some(SpecSurface::TCL86_PLUS),
        ..SubCommandForm::DEFAULT
    },
];

// ---------------------------------------------------------------------------
// `namespace ensemble`'s two option tables
// ---------------------------------------------------------------------------
// `namespace ensemble create` and `namespace ensemble configure` are two
// **different** option tables, not one shared table (issue #1610).
//
// C Tcl declares them side by side in `tclEnsemble.c` and dispatches each
// through its own `Tcl_GetIndexFromObj` — `ensembleCreateOptions` is
// `-command -map -parameters -prefixes -subcommands -unknown`,
// `ensembleConfigOptions` is `-map -namespace -parameters -prefixes
// -subcommands -unknown`. They differ at exactly two entries, in opposite
// directions, so each table's distinctive option is the *other* one's error.
// Pinned on tclsh 8.6.16 and 9.0.4 (byte identical):
//
// ```text
// namespace ensemble configure ::E -namespace  → ::M
// namespace ensemble configure ::E -command x  → bad option "-command": must be
//     -map, -namespace, -parameters, -prefixes, -subcommands, or -unknown
// namespace ensemble create -namespace ::M     → bad option "-namespace": must be
//     -command, -map, -parameters, -prefixes, -subcommands, or -unknown
// ```
//
// `-namespace` is *readable* but not settable: the setter path's
// `CONF_NAMESPACE` arm answers `option -namespace is read-only`
// (`TCL ENSEMBLE READ_ONLY`) rather than `bad option`, which is why it
// belongs in `configure`'s table — a query (`configure ::E -namespace`) is
// a perfectly ordinary use of it, and a Tk-library idiom. The earlier merged
// table reasoned from this project's own older runtime code instead of from
// C Tcl, and so simultaneously offered `configure` a `-command` that always
// errors and hid the `-namespace` it accepts.
//
// Every option in both tables except `-namespace` takes a single value word:
// `create` and `configure`'s update form both parse strict `-option value`
// pairs, no bare flags. `-namespace` is the exception because it has no
// update form at all — see [`ENSEMBLE_OPT_NAMESPACE`].
//
// Dialect gating below is checked directly against the Tcl 8.5, 8.6,
// 9.0, and 9.1 `namespace.n` manpages (this project's own
// implementations don't themselves version-gate any option, so they
// can't be the source of truth here): `-command`, `-map`, `-namespace`,
// `-prefixes`, `-subcommands`, and `-unknown` are present from 8.5 (when
// `namespace ensemble` itself was introduced) onward and so inherit the
// subcommand's own `TCL85_PLUS` gate. `-parameters` is the one
// exception — it is absent from the Tcl 8.5 ENSEMBLE OPTIONS list
// (missing from both the option table and the `-map`/`-prefixes`/
// `-subcommands`/`-unknown`/`-command`/`-namespace` enumeration) and
// first appears in the Tcl 8.6 manpage, so it carries its own,
// narrower `TCL86_PLUS` gate.
/// `-command`: in `ensembleCreateOptions` only.
const ENSEMBLE_OPT_COMMAND: OptionSpec = OptionSpec {
    name: "-command",
    value: OptionValue::value("name"),
    detail: "Name of the ensemble's dispatch command (default: the fully-qualified name of the invoking namespace). Write-only, and valid only with create — configure rejects it as a bad option.",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
};

/// `-namespace`: in `ensembleConfigOptions` only, and read-only even there —
/// `configure ::E -namespace` answers with the linked namespace, while
/// `configure ::E -namespace ::M` raises `option -namespace is read-only`
/// (`CONF_NAMESPACE`, `tclEnsemble.c`).
const ENSEMBLE_OPT_NAMESPACE: OptionSpec = OptionSpec {
    name: "-namespace",
    // Value-less: `-namespace`'s only legal use is `namespace ensemble
    // configure CMD -namespace`, the query form, which takes no value word.
    // The setter form that would take one is the form that always raises
    // `option -namespace is read-only`, so declaring an arity for it would
    // describe the error rather than the option.
    value: OptionValue::flag(),
    detail: "The namespace the ensemble dispatches into, fixed when it was created. Read it back with configure; supplying a value raises \"option -namespace is read-only\", and create rejects it as a bad option.",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
};

/// The five options both C tables carry, in their shared (alphabetical) order.
const ENSEMBLE_OPT_MAP: OptionSpec = OptionSpec {
    name: "-map",
    value: OptionValue::value("dict"),
    detail: "Maps subcommand names to target command-prefix lists, similar to interp alias (default: empty, meaning each subcommand maps to the identically-named command in the linked namespace).",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
};

const ENSEMBLE_OPT_PARAMETERS: OptionSpec = OptionSpec {
    name: "-parameters",
    value: OptionValue::value("list"),
    detail: "Named arguments inserted between the ensemble command and the subcommand, used when generating error messages (default: none).",
    surface: Some(SpecSurface::TCL86_PLUS),
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
};

const ENSEMBLE_OPT_PREFIXES: OptionSpec = OptionSpec {
    name: "-prefixes",
    value: OptionValue::boolean(),
    detail: "Whether unambiguous subcommand prefixes are accepted (default: on).",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
};

const ENSEMBLE_OPT_SUBCOMMANDS: OptionSpec = OptionSpec {
    name: "-subcommands",
    value: OptionValue::value("list"),
    detail: "Explicit list of valid subcommand names (default: empty, meaning the -map keys or the linked namespace's exported commands).",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
};

const ENSEMBLE_OPT_UNKNOWN: OptionSpec = OptionSpec {
    name: "-unknown",
    value: OptionValue::deferred_command_prefix("prefix"),
    detail: "Command prefix invoked, with the ensemble's own invocation words appended, when a subcommand is not recognised (default: none, which raises a standard \"unknown subcommand\" error).",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
};

/// A namespace unknown handler is installed for a future failed dispatch;
/// setting or querying it never invokes the prefix in this call.
fn namespace_unknown_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (!args.is_empty())
        .then_some((0, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}

/// `ensembleCreateOptions` (`tclEnsemble.c`), in C's own order.
static ENSEMBLE_CREATE_OPTIONS: &[OptionSpec] = &[
    ENSEMBLE_OPT_COMMAND,
    ENSEMBLE_OPT_MAP,
    ENSEMBLE_OPT_PARAMETERS,
    ENSEMBLE_OPT_PREFIXES,
    ENSEMBLE_OPT_SUBCOMMANDS,
    ENSEMBLE_OPT_UNKNOWN,
];

/// `ensembleConfigOptions` (`tclEnsemble.c`), in C's own order — `-namespace`
/// where `create` has `-command`.
static ENSEMBLE_CONFIG_OPTIONS: &[OptionSpec] = &[
    ENSEMBLE_OPT_MAP,
    ENSEMBLE_OPT_NAMESPACE,
    ENSEMBLE_OPT_PARAMETERS,
    ENSEMBLE_OPT_PREFIXES,
    ENSEMBLE_OPT_SUBCOMMANDS,
    ENSEMBLE_OPT_UNKNOWN,
];

/// The union of both tables, carried on the `ensemble` subcommand itself for
/// the **abstain** path only: a call whose dispatch word is dynamic
/// (`namespace ensemble $op …`) or not yet typed, where no consumer can say
/// which of the two tables applies.
///
/// A union is the right answer there and the wrong one anywhere else. It
/// cannot draw a false "not in the table" for either operation, which is what
/// a consumer facing an unknown dispatch word needs; but it also offers both
/// `-command` and `-namespace`, exactly one of which is always an error, so
/// every consumer that *can* see the dispatch word must take the narrower
/// table through [`SubCommand::option_scope`](crate::SubCommand::option_scope)
/// instead (issue #1610).
static ENSEMBLE_ANY_OPTIONS: &[OptionSpec] = &[
    ENSEMBLE_OPT_COMMAND,
    ENSEMBLE_OPT_MAP,
    ENSEMBLE_OPT_NAMESPACE,
    ENSEMBLE_OPT_PARAMETERS,
    ENSEMBLE_OPT_PREFIXES,
    ENSEMBLE_OPT_SUBCOMMANDS,
    ENSEMBLE_OPT_UNKNOWN,
];

/// `namespace which ?-command? ?-variable? name` — the two leading flags select
/// what to resolve `name` as. They are bare (value-less) flags declared in
/// `WHICH_OPTIONS`, so the arity check skips them and `exact(1)` counts only the
/// trailing `name` (catching `namespace which foo bar`).
///
/// `namespace which` is an existence **probe**: it returns `""` for an unknown
/// name rather than failing.  Under `-variable` the positional `name` is a
/// `VarRead` reference — navigation only, and variables draw no
/// unknown-variable diagnostic, so a probe of an absent variable is harmless.
///
/// The default / `-command` form is a [`ArgRole::CommandNameProbe`]
/// reference (issue #945 fault 9): the name navigates — find-references /
/// go-to-definition / rename reach it like any direct reference — while
/// the probe existence policy keeps it out of the W123 unresolved-command
/// pass, so a perfectly valid existence check
/// (`if {[namespace which -command foo] eq ""} …`) of a command the
/// analyser cannot see is never flagged.  `args` are the words after the
/// `which` subcommand.
fn namespace_which_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let is_variable = args
        .iter()
        .any(|a| a.len() > 1 && "-variable".starts_with(*a));
    let Some(idx) = args.iter().rposition(|a| !a.starts_with('-')) else {
        return Vec::new();
    };
    let role = if is_variable {
        ArgRole::VarRead
    } else {
        ArgRole::CommandNameProbe
    };
    u8::try_from(idx).map_or_else(|_| Vec::new(), |i| vec![(i, role)])
}

static WHICH_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        value: OptionValue::flag(),
        detail: "Resolve name as a command (the default).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::flag(),
        detail: "Resolve name as a variable.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// `namespace ensemble`'s own dispatch word (`create`/`configure`/
/// `exists`) — a single bare word in every version that documents
/// `namespace ensemble` at all (8.5, 8.6, 9.0, 9.1 all enumerate exactly
/// these three and no others), never a list, so exact-word closure is
/// safe here — unlike `open`'s `ACCESS_VALUES`, which must stay open
/// because its argument can legitimately be a multi-word list.
const ENSEMBLE_OP_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "create",
        detail: "Create a new ensemble command linked to the current namespace.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "configure",
        detail: "Query or update the options of an existing ensemble command.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "exists",
        detail: "Test whether a command exists and is an ensemble command.",
        ..ArgValue::DEFAULT
    },
];

/// The second-level dispatcher under `namespace ensemble`.  This is distinct
/// from the ensemble command it creates: these are the three management
/// operations implemented by Tcl's own `namespace` ensemble.  Keeping them
/// in [`SubCommand::sub_subcommands`] lets generic completion, hover, and
/// semantic-token consumers recognise the word after `ensemble`, including
/// Tcl's unique-prefix rule, without any consumer knowing the `namespace`
/// command by name.
///
/// Tcl 9.0.4's `namespace(n)` and the Tcl 8.5 executable agree on this exact
/// set.  The enclosing `ensemble` subcommand is `TCL85_PLUS`, so these inherit
/// that gate rather than repeating it on every entry.
const ENSEMBLE_SUB_SUBCOMMANDS: &[SubSubCommand] = &[
    SubSubCommand {
        name: "create",
        detail: "Create an ensemble command for the current namespace.",
        synopsis: "namespace ensemble create ?-option value ...?",
        options: Some(ENSEMBLE_CREATE_OPTIONS),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "configure",
        detail: "Query or update an existing ensemble command.",
        synopsis: "namespace ensemble configure command ?-option? ?value ...?",
        options: Some(ENSEMBLE_CONFIG_OPTIONS),
        ..SubSubCommand::DEFAULT
    },
    SubSubCommand {
        name: "exists",
        // `exists` takes **no options**, and says so explicitly rather than
        // inheriting the parent's union. `ENS_EXISTS` (`tclEnsemble.c`) is
        // `if (objc != 3) { Tcl_WrongNumArgs(…, "cmdname"); }` and then
        // `Tcl_FindEnsemble` — it never reaches an option table, so a
        // `-`-shaped word here is the *command name*, not a flag. Pinned on
        // tclsh 8.6.16 and 9.0.4, byte identical:
        //
        //     namespace ensemble exists -namespace      → 0
        //     namespace ensemble exists -namespace foo  → wrong # args: should be
        //         "namespace ensemble exists cmdname"
        //
        // The first line is why the empty table has to be explicit: offering
        // `-namespace` here does not merely suggest a rejected option, it
        // suggests a word that silently changes the call into "is there an
        // ensemble named `-namespace`".
        detail: "Return whether command is an ensemble command.",
        synopsis: "namespace ensemble exists command",
        options: Some(&[]),
        ..SubSubCommand::DEFAULT
    },
];

/// `namespace export`'s only flag — present unchanged in the synopsis of
/// every fetched version (8.4 through 9.1).
static EXPORT_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-clear",
    value: OptionValue::flag(),
    detail: "Reset the namespace's export pattern list to empty before appending the given patterns.",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
}];

/// `namespace import`'s only flag — present unchanged in the synopsis of
/// every fetched version (8.4 through 9.1).
static IMPORT_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-force",
    value: OptionValue::flag(),
    detail: "Silently overwrite an existing command instead of erroring on conflict.",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
}];

/// `namespace delete ?namespace namespace ...?` — every positional word names
/// a namespace, so the role is variadic and cannot be a fixed index table.
///
/// `NamespaceDeleteCmd` (tclNamesp.c) walks `objv[1..]` and deletes each,
/// erroring on the first unknown one (`unknown namespace "::never" in
/// namespace delete command`, rc 1 — identical on tclsh 9.0.4 and 8.6.16), so
/// there is no flag or terminator word to skip. `args` are the words after
/// the `delete` subcommand.
fn namespace_delete_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    (0..args.len())
        .filter_map(|i| u8::try_from(i).ok())
        .map(|i| (i, ArgRole::NamespaceName))
        .collect()
}

/// Compile-time folds for `namespace qualifiers` / `namespace tail`
/// (issue #1096), consumed by the optimiser's O129 general-builtin
/// constant-fold path through the registry `const_fold` callbacks.
///
/// Both are **pure string operations**: `namespace.n` describes them as
/// splitting a *string* at its last `::` separator, and neither consults the
/// interpreter's namespace table — a name for a namespace that does not
/// exist splits exactly the same way as one that does.  So the fold needs no
/// namespace-existence check and is namespace-context-independent, which is
/// what makes it sound to fire on any provably-constant word (including the
/// `[self class]` frame constant the O129 path resolves first — that chain is
/// the point of the issue).
///
/// The two functions are byte-exact ports of `NamespaceQualifiersCmd` /
/// `NamespaceTailCmd` (`tclNamesp.c`), scanning bytes backwards for the last
/// `::`.  Pinned against tclsh 9.0.4 and 8.6.14, byte-identical on every row
/// (including the four edge cases issue #1096 tabulates), by the unit tests
/// below and the `tclsh`-differential matrix in
/// `tests/differential_fold.rs`.  Working on bytes is UTF-8-safe here because
/// `:` is ASCII and can never occur inside a multi-byte sequence: every cut
/// point is immediately before or after an ASCII `:`, hence always a char
/// boundary.
///
/// Registered dialect-invariantly (`const_fold`, not `const_fold_versioned`):
/// the C implementation of both subcommands is unchanged across 8.4-9.1 and
/// the transcripts agree byte-for-byte.
fn fold_qualifiers(args: &[&str]) -> Option<String> {
    let [s] = args else {
        return None;
    };
    Some(crate::state_transition::namespace_qualifiers(s).to_owned())
}

fn fold_tail(args: &[&str]) -> Option<String> {
    let [s] = args else {
        return None;
    };
    let b = s.as_bytes();
    if b.is_empty() {
        // `NamespaceTailCmd`'s `p` lands before the string start for the
        // empty input and sets no result at all — the empty string.
        return Some(String::new());
    }
    // The scan stops one short of the front (C's `while (--p > name)`), so a
    // leading `::` is never treated as a separator: `namespace tail ::foo` is
    // `foo`, but `namespace tail :` is `:`.
    let mut p = b.len() - 1;
    let mut start = 0usize;
    while p > 0 {
        if b[p] == b':' && b[p - 1] == b':' {
            start = p + 1;
            break;
        }
        p -= 1;
    }
    Some(s[start..].to_owned())
}

const NAMESPACE_UPVAR_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::VariableTraces,
];

const NAMESPACE_LOOKUP_TRANSITION_DOMAINS: &[StateTransitionDomain] =
    &[StateTransitionDomain::Namespaces];

const NAMESPACE_IMPORT_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::CommandBindings,
    StateTransitionDomain::Namespaces,
];

const NAMESPACE_DELETE_TRANSITION_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::CommandBindings,
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::Namespaces,
    StateTransitionDomain::CommandTraces,
    StateTransitionDomain::ExecutionTraces,
    StateTransitionDomain::VariableTraces,
    StateTransitionDomain::ObjectDispatch,
];

/// The parent `namespace` spec retains its long-standing namespace-state
/// side effect.  A selected namespace transition is the completion-qualified,
/// target-specific account of that write, so only that legacy write is handed
/// to the transition projection.
const NAMESPACE_LOOKUP_EFFECT_COVERAGE: &[TransitionEffectCoverage] = &[TransitionEffectCoverage {
    source: WorldEffectWriteSource::LegacySideEffect(SideEffectTarget::NamespaceState),
    domains: &[WorldStateDomain::NamespaceLookup],
}];

const NAMESPACE_DELETE_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: &[],
        // Deleting a namespace recursively deletes its commands.  Command
        // delete traces can synchronously re-enter the interpreter while the
        // tree is only partly destroyed.
        callback: CallbackEffect {
            kinds: CallbackKinds::TRACE,
            reentrancy: Reentrancy::CurrentInterpreter,
        },
    },
    resolver: None,
    dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const NAMESPACE_EVAL_EFFECTS: WorldEffectDescriptor = WorldEffectDescriptor {
    composition: WorldEffectComposition::Extend,
    static_footprint: StaticEffectFootprint {
        accesses: &[],
        // The body runs after namespace creation and may return any Tcl
        // completion, so the generic callback barrier is required even when
        // the namespace operand itself is known.
        callback: CallbackEffect {
            kinds: CallbackKinds::SCRIPT,
            reentrancy: Reentrancy::CurrentInterpreter,
        },
    },
    resolver: None,
    dynamic_fallback: WorldEffectDynamicFallback::ConservativeUnknownInvocation,
};

const NAMESPACE_DELETE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_delete_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::EveryArgument,
        domains: NAMESPACE_DELETE_TRANSITION_DOMAINS,
    }],
    effect_coverage: NAMESPACE_LOOKUP_EFFECT_COVERAGE,
    // Tcl destroys each requested tree in turn.  A later unknown namespace or
    // an observer callback can therefore report an abrupt completion after a
    // preceding tree has disappeared.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

const NAMESPACE_ENSEMBLE_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_ensemble_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::EveryArgument,
        domains: NAMESPACE_IMPORT_TRANSITION_DOMAINS,
    }],
    effect_coverage: NAMESPACE_LOOKUP_EFFECT_COVERAGE,
    // Ensemble construction/configuration validates several options after it
    // has selected mutable ensemble state; preserve the intermediate state on
    // an abrupt completion conservatively.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

const NAMESPACE_EVAL_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_eval_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[1]),
        domains: NAMESPACE_LOOKUP_TRANSITION_DOMAINS,
    }],
    effect_coverage: NAMESPACE_LOOKUP_EFFECT_COVERAGE,
    // Namespace creation happens before the body executes.  The body can
    // fail, return, or re-enter Tcl after the namespace is visible.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

const NAMESPACE_EXPORT_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_export_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::EveryArgument,
        domains: NAMESPACE_LOOKUP_TRANSITION_DOMAINS,
    }],
    effect_coverage: NAMESPACE_LOOKUP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const NAMESPACE_FORGET_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_forget_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::EveryArgument,
        domains: NAMESPACE_IMPORT_TRANSITION_DOMAINS,
    }],
    effect_coverage: NAMESPACE_LOOKUP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

const NAMESPACE_IMPORT_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_import_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::EveryArgument,
        domains: NAMESPACE_IMPORT_TRANSITION_DOMAINS,
    }],
    effect_coverage: NAMESPACE_LOOKUP_EFFECT_COVERAGE,
    // Imports are installed pattern by pattern, and `-force` can replace a
    // binding before a later pattern errors.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

const NAMESPACE_PATH_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_path_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[1]),
        domains: NAMESPACE_LOOKUP_TRANSITION_DOMAINS,
    }],
    effect_coverage: NAMESPACE_LOOKUP_EFFECT_COVERAGE,
    commit: StateTransitionCommit::OnOkOnly,
};

const NAMESPACE_UNKNOWN_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_unknown_state_transitions),
    argument_shape: StateTransitionArgumentShape::Independent,
    dynamic_widening: &[StateTransitionWideningRule {
        operands: StateTransitionOperandLayout::Indices(&[1]),
        domains: NAMESPACE_LOOKUP_TRANSITION_DOMAINS,
    }],
    // The legacy namespace-state bridge is a namespace-lookup write, while
    // this transition names the distinct fallback-handler partition.  Keep
    // the legacy write visible rather than claiming it is covered.
    effect_coverage: TransitionEffectCoverage::NONE,
    commit: StateTransitionCommit::OnOkOnly,
};

const NAMESPACE_UPVAR_TRANSITIONS: StateTransitionDescriptor = StateTransitionDescriptor {
    composition: StateTransitionComposition::Extend,
    resolver: Some(namespace_upvar_state_transitions),
    argument_shape: StateTransitionArgumentShape::Positional,
    dynamic_widening: &[StateTransitionWideningRule {
        // The resolved `upvar` word itself is index 0. Every remaining word
        // participates in the namespace/otherVar/myVar identity grammar.
        operands: StateTransitionOperandLayout::EveryArgument,
        domains: NAMESPACE_UPVAR_TRANSITION_DOMAINS,
    }],
    // `namespace upvar` has no duplicate legacy world write: its transition
    // is the sole identity fact, so it deliberately covers nothing.
    effect_coverage: TransitionEffectCoverage::NONE,
    // Namespace aliases are established pair by pair and can be visible to
    // variable traces before a later pair reports an error.
    commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
};

fn namespace_upvar_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let Some(namespace) = TransitionSubject::from_argument(arguments, 1) else {
        return transitions;
    };
    for other_index in (2..arguments.len()).step_by(2) {
        let (Some(variable), Some(local)) = (
            TransitionSubject::from_argument(arguments, other_index),
            TransitionSubject::from_argument(arguments, other_index + 1),
        ) else {
            continue;
        };
        transitions.push(StateTransition::VariableCellAlias(
            VariableCellAliasTransition {
                local,
                target: VariableAliasTarget::Namespace {
                    namespace: namespace.clone(),
                    variable,
                },
            },
        ));
    }
    transitions
}

fn current_namespace() -> NamespaceTransitionTarget {
    NamespaceTransitionTarget::Current
}

fn named_namespace(arguments: InvocationArguments<'_>) -> Option<NamespaceTransitionTarget> {
    TransitionSubject::from_argument(arguments, 1).map(NamespaceTransitionTarget::Named)
}

fn subjects_from(arguments: InvocationArguments<'_>, first: usize) -> Vec<TransitionSubject> {
    (first..arguments.len())
        .filter_map(|index| TransitionSubject::from_argument(arguments, index))
        .collect()
}

fn is_leading_option(arguments: InvocationArguments<'_>, option: &str) -> bool {
    arguments
        .literal_at(1)
        .is_some_and(|value| !value.is_empty() && option.starts_with(value))
}

fn namespace_delete_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    for namespace in subjects_from(arguments, 1) {
        transitions.push(StateTransition::Namespace(NamespaceTransition::Delete {
            namespace: NamespaceTransitionTarget::Named(namespace),
        }));
    }
    transitions
}

fn namespace_ensemble_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    // `exists` is the sole documented read-only ensemble operation.  Other
    // literal spellings include accepted abbreviations, and an invalid
    // spelling may still have selected mutable ensemble state before it
    // errors, so retain the conservative mutation fact for all of them.
    if arguments.len() > 1 && arguments.literal_at(1) != Some("exists") {
        transitions.push(StateTransition::Namespace(NamespaceTransition::Ensemble {
            namespace: current_namespace(),
        }));
    }
    transitions
}

fn namespace_eval_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let Some(namespace) = named_namespace(arguments) else {
        return transitions;
    };
    transitions.push(StateTransition::Namespace(NamespaceTransition::Ensure {
        namespace,
    }));
    transitions
}

fn namespace_export_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    // `-clear` is itself a mutation even when it is the only argument, so it
    // stays in the same whole-Tcl-value operand list as ordinary patterns.
    let patterns = subjects_from(arguments, 1);
    if !patterns.is_empty() {
        transitions.push(StateTransition::Namespace(NamespaceTransition::Export {
            namespace: current_namespace(),
            patterns,
        }));
    }
    transitions
}

fn namespace_forget_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let patterns = subjects_from(arguments, 1);
    if !patterns.is_empty() {
        transitions.push(StateTransition::Namespace(NamespaceTransition::Forget {
            namespace: current_namespace(),
            patterns,
        }));
    }
    transitions
}

fn namespace_import_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let first_pattern = if is_leading_option(arguments, "-force") {
        2
    } else {
        1
    };
    let patterns = subjects_from(arguments, first_pattern);
    if !patterns.is_empty() {
        transitions.push(StateTransition::Namespace(NamespaceTransition::Import {
            namespace: current_namespace(),
            patterns,
        }));
    }
    transitions
}

fn namespace_path_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let Some(path) = TransitionSubject::from_argument(arguments, 1) else {
        return transitions;
    };
    transitions.push(StateTransition::Namespace(NamespaceTransition::SetPath {
        namespace: current_namespace(),
        path,
    }));
    transitions
}

fn namespace_unknown_state_transitions(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    let Some(handler) = TransitionSubject::from_argument(arguments, 1) else {
        return transitions;
    };
    transitions.push(StateTransition::Namespace(
        NamespaceTransition::SetUnknown {
            namespace: current_namespace(),
            handler,
        },
    ));
    transitions
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "children",
        arity: Arity::new(0, 2),
        detail: "Returns a list of all child namespaces.",
        synopsis: "namespace children ?namespace? ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        // The optional first word names the namespace whose children are
        // listed (`namespace children ::tomato` — issue #1088); the optional
        // second is a glob pattern filtering the *result*, not a namespace.
        arg_roles: &[(0, ArgRole::NamespaceName), (1, ArgRole::Pattern)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "code",
        arity: Arity::exact(1),
        detail: "Captures the current namespace context for later execution.",
        synopsis: "namespace code script",
        pure: true,
        return_type: Some(TclType::String),
        // The captured script runs in the *current* namespace when the
        // callback fires, so analyse it in this scope — a `Body` — for its
        // references / definitions.
        arg_roles: &[(0, ArgRole::Body)],
        // …and the value it returns is itself a command prefix
        // (`::namespace inscope NS script`), so a consumer sitting in a
        // command-prefix position — `trace add variable v w [namespace code
        // [list Tracer]]`, Tk's own `fontchooser.tcl` idiom — unwraps this
        // one level and extracts the head from the script. See
        // `Traits::WRAPS_COMMAND_PREFIX`.
        traits: Traits::WRAPS_COMMAND_PREFIX,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "current",
        arity: Arity::exact(0),
        detail: "Returns the fully-qualified name for the current namespace.",
        synopsis: "namespace current",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::any(),
        detail: "Delete namespaces and their contents.",
        synopsis: "namespace delete ?namespace namespace ...?",
        // `Tcl_NamespaceObjCmd` (tclNamesp.c, `NamespaceDeleteCmd` →
        // `Tcl_DeleteNamespace`) destroys the namespace with everything in
        // it and errors on an unknown namespace — `catch {namespace delete
        // $ns}` is the documented fire-and-forget idiom the W302
        // suppression keys off.
        destructive: true,
        return_type: Some(TclType::String),
        // Every positional word names a namespace — see
        // `namespace_delete_arg_roles`.
        arg_role_resolver: Some(namespace_delete_arg_roles),
        world_effects: Some(NAMESPACE_DELETE_EFFECTS),
        state_transitions: Some(NAMESPACE_DELETE_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ensemble",
        arity: Arity::at_least(1),
        detail: "Creates and manipulates a command ensemble.",
        synopsis: "namespace ensemble subcommand ?arg ...?",
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL85_PLUS),
        // The abstain-path table only: `create` and `configure` carry their
        // own, genuinely different tables on `ENSEMBLE_SUB_SUBCOMMANDS`, and
        // a consumer that can read the dispatch word takes those. This union
        // is what is left when the dispatch word is dynamic or absent — see
        // `ENSEMBLE_ANY_OPTIONS` (issue #1610).
        options: ENSEMBLE_ANY_OPTIONS,
        // The dispatch word itself (index 0, right after `ensemble`) is a
        // closed 3-word enum in every version that has `namespace
        // ensemble` at all — see `ENSEMBLE_OP_VALUES`. Like the top-level
        // `namespace` dispatch ("you can abbreviate the subcommands"),
        // this second-level dispatch also accepts a unique prefix —
        // confirmed empirically against real `tclsh` 8.6.14: both
        // `namespace ensemble cre` and `namespace ensemble conf` resolve
        // (to `create`/`configure`) exactly like the unabbreviated forms.
        arg_values: &[(0, ENSEMBLE_OP_VALUES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
        sub_subcommands: ENSEMBLE_SUB_SUBCOMMANDS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceEnsemble),
        // An ensemble publishes its namespace's commands under a single
        // dispatching command name (and `-map` can redirect a subcommand
        // to an arbitrary prefix), so the mapped commands acquire callers
        // that need not appear in this file at all.  The map / dispatch
        // machinery also holds command names as data, so a program using
        // ensembles observes command names (REFLECTS_COMMAND_NAMES).
        traits: Traits::EXPORTS_COMMAND.union(Traits::REFLECTS_COMMAND_NAMES),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(NAMESPACE_ENSEMBLE_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::at_least(2),
        detail: "Evaluate a script in a namespace context.",
        synopsis: "namespace eval namespace arg ?arg ...?",
        // The target word is a namespace **name**, not a generic symbolic
        // `Name`: it is the one form that *declares* a namespace (see
        // `Traits::DECLARES_NAMESPACE` below), and every other spelling of
        // the same namespace — `namespace children ::ns`, `namespace exists
        // ns` — must navigate to it (issue #1088).
        arg_roles: &[(0, ArgRole::NamespaceName), (1, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::NamespaceEval),
        return_type: Some(TclType::String),
        // The body evaluates in the *namespace* frame, not the caller's, so
        // SSA must not recover its `$var` reads as caller-local reads (else a
        // proc param read only inside the body looks used).  The body's
        // `$var` reads are excluded from caller-local read recovery.
        body_kind: BodyKind::Structural,
        // Runs an arbitrary script that can touch namespace/global state —
        // dynamic-dispatch consumers (memory-SSA clobber classification)
        // key off this.  Words after the body concatenate into it exactly as
        // `eval`'s do (`namespace eval ::n set l2 hello` sets `::n::l2`,
        // tclsh8.6.14/9.0.4-confirmed), so the eval-family trait applies.
        // `DECLARES_NAMESPACE` is what makes this the namespace's *declaring*
        // site rather than merely another reference to it.  Pinned on tclsh
        // 9.0.4 and 8.6.16 (byte-identical): `namespace eval ::a {}` twice
        // creates the namespace once and extends it the second time, a deep
        // `namespace eval ::p::q::r {}` creates `::p` and `::p::q` too, and
        // it is the **only** declaring form — `proc ::nope::gone::p {} {}`
        // with no such namespace fails (`can't create procedure
        // "::nope::gone::p": unknown namespace`, rc 1), as does `set
        // ::brandnew::v 1` (`parent namespace doesn't exist`).  `inscope`,
        // which shares this hook and arg layout, deliberately does **not**
        // carry it: it requires the namespace to already exist.
        traits: Traits::EVALUATES_CODE
            .union(Traits::SCRIPT_CONCATENATES_ARGS)
            .union(Traits::DECLARES_NAMESPACE),
        world_effects: Some(NAMESPACE_EVAL_EFFECTS),
        state_transitions: Some(NAMESPACE_EVAL_TRANSITIONS),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceEval),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Test whether a namespace exists.",
        synopsis: "namespace exists namespace",
        pure: true,
        return_type: Some(TclType::Boolean),
        // An existence probe of a namespace: the name navigates like any
        // other namespace reference, and no diagnostic asserts it exists
        // (both interpreters answer `0` rather than erroring).
        arg_roles: &[(0, ArgRole::NamespaceName)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "export",
        arity: Arity::any(),
        detail: "Specifies which commands are exported from a namespace; with no patterns and no -clear, returns the namespace's current export list.",
        synopsis: "namespace export ?-clear? ?pattern pattern ...?",
        return_type: Some(TclType::List),
        options: EXPORT_OPTIONS,
        // `NamespaceExportCmd` compares `objv[1]` against `-clear` once and
        // never again, so a second `-clear` is an ordinary export *pattern*
        // (tclsh 8.6.14/9.0.4: `namespace export -clear -clear p` leaves
        // `-clear p` exported, and `-clear` is then a genuinely importable
        // command name). Likewise `namespace export a -clear` exports both —
        // the flag is only ever the first word.
        max_leading_option_words: Some(1),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceExport),
        // Publishes this namespace's commands for another unit to
        // `namespace import`, so the exported names have callers this file
        // does not contain.  The export patterns match commands by their
        // spelled names, so those names are observable data
        // (REFLECTS_COMMAND_NAMES).
        traits: Traits::EXPORTS_COMMAND.union(Traits::REFLECTS_COMMAND_NAMES),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(NAMESPACE_EXPORT_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        // Removes imported commands matched by spelled name / pattern.
        traits: Traits::FIRE_AND_FORGET_TEARDOWN.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::any(),
        detail: "Removes previously imported commands from a namespace.",
        synopsis: "namespace forget ?pattern pattern ...?",
        // `NamespaceForgetCmd` (tclNamesp.c → `Tcl_ForgetImport`) removes
        // previously imported command aliases — a removal of interpreter
        // state, so `catch {namespace forget …}` is treated as the same
        // fire-and-forget idiom as `namespace delete` by the W302
        // suppression.
        destructive: true,
        return_type: Some(TclType::String),
        // The removal half of the import edge's lifecycle: the analyser
        // records each pattern as an ordered event so a bare call written
        // after the forget stops resolving through the alias it removed
        // (issue #1103; `namespace import`'s own hook is the install half).
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceForget),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(NAMESPACE_FORGET_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "import",
        // Imports commands by their spelled names / patterns.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::any(),
        // The bare (no-pattern, no-flag) query form — "returns the list of
        // commands in the current namespace that have been imported from
        // other namespaces" — is documented starting with the Tcl 8.5
        // manpage; the Tcl 8.4 manpage only describes the importing
        // behaviour, with no query-form return value stated for a
        // zero-argument call. `import` itself has no dialect gate (it is
        // present in 8.4 too), so this is a behavioural note rather than a
        // `surface:` restriction — see the module-level task guidance on
        // "exists everywhere but behaviour changed at some version".
        detail: "Imports commands into a namespace; with no arguments, returns the list of commands already imported into the current namespace (Tcl 8.5+).",
        synopsis: "namespace import ?-force? ?pattern pattern ...?",
        return_type: Some(TclType::List),
        options: IMPORT_OPTIONS,
        // As for `export`: `NamespaceImportCmd` consumes at most one leading
        // `-force`. A second one is read as an import pattern and aborts the
        // script (tclsh 8.6.14/9.0.4: `namespace import -force -force
        // ::src::*` → `no namespace specified in import pattern "-force"`),
        // as does a trailing one (`namespace import ::src::p -force`).
        max_leading_option_words: Some(1),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceImport),
        // Deliberately NOT `LOADS_EXTERNAL_UNIT`: `namespace import
        // ::lib::*` is just as often an intra-file convenience over a
        // namespace the same file defines, which proves nothing about
        // other units. The import *is* a real caller path, but
        // `unit_scope`'s evidence scan already models it precisely (it
        // resolves calls through the import, and records an opaque caller
        // when it cannot), so nothing is lost by leaving the coarse
        // boundary flag off.
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(NAMESPACE_IMPORT_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inscope",
        arity: Arity::at_least(2),
        detail: "Executes a script in the context of the specified namespace.",
        synopsis: "namespace inscope namespace script ?arg ...?",
        // Same shape as `eval`'s, and the same namespace **reference** —
        // but `inscope` requires the namespace to exist already, so it
        // carries no `DECLARES_NAMESPACE`.
        arg_roles: &[(0, ArgRole::NamespaceName), (1, ArgRole::Body)],
        return_type: Some(TclType::String),
        // Like `eval`, the script runs in the namespace frame — the `[subcmd,
        // namespace, body]` shape is identical, so the same analyser hook
        // opens the namespace scope and walks the body there (rather than the
        // caller's scope, where a bare command would resolve wrongly).
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceEval),
        body_kind: BodyKind::Structural,
        // In the eval family (the trailing words are script, not options),
        // but with the list-append refinement: they arrive as whole list
        // elements rather than space-joined text, so no consumer may join
        // them.  See `Traits::SCRIPT_APPENDS_LIST_ARGS`.
        traits: Traits::EVALUATES_CODE
            .union(Traits::SCRIPT_CONCATENATES_ARGS)
            .union(Traits::SCRIPT_APPENDS_LIST_ARGS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "origin",
        // Resolves an imported command's original spelled name.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::exact(1),
        detail: "Returns the fully-qualified name of the original command.",
        synopsis: "namespace origin command",
        pure: true,
        return_type: Some(TclType::String),
        // The single argument is a command name resolved (not called), so it
        // is a command reference navigation follows.
        arg_roles: &[(0, ArgRole::CommandName)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "parent",
        arity: Arity::new(0, 1),
        detail: "Returns the fully-qualified name of the parent namespace.",
        synopsis: "namespace parent ?namespace?",
        pure: true,
        return_type: Some(TclType::String),
        // The optional word names the namespace whose parent is reported.
        arg_roles: &[(0, ArgRole::NamespaceName)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "path",
        arity: Arity::new(0, 1),
        detail: "Returns the command resolution path of the current namespace.",
        synopsis: "namespace path ?namespaceList?",
        return_type: Some(TclType::List),
        surface: Some(SpecSurface::TCL85_PLUS),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespacePath),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(NAMESPACE_PATH_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "qualifiers",
        arity: Arity::exact(1),
        detail: "Returns any leading namespace qualifiers for string.",
        synopsis: "namespace qualifiers string",
        pure: true,
        return_type: Some(TclType::String),
        // Pure string arithmetic on the word — see [`fold_qualifiers`].
        const_fold: Some(fold_qualifiers),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tail",
        arity: Arity::exact(1),
        detail: "Returns the simple name at the end of a qualified string.",
        synopsis: "namespace tail string",
        pure: true,
        return_type: Some(TclType::String),
        // Pure string arithmetic on the word — see [`fold_tail`].
        const_fold: Some(fold_tail),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unknown",
        // Installs a handler that receives unresolved command names.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::new(0, 1),
        detail: "Sets or returns the unknown command handler for the current namespace.",
        synopsis: "namespace unknown ?script?",
        return_type: Some(TclType::String),
        // Added in 8.5 (`NamespaceUnknownCmd`, tclNamesp.c), like the sibling
        // `namespace path`; 8.4's `namespace` ensemble has no `unknown`
        // subcommand.  Gate it the same as `path` so an 8.4 document flags it.
        surface: Some(SpecSurface::TCL85_PLUS),
        // The optional handler (index 0 after `unknown` → arg 1) is a command
        // prefix invoked with the unknown command name + its args appended
        // (variadic ⇒ AtLeast(1)). The zero-arg query form has no prefix.
        command_prefixes: &[(0, AppendedArity::AtLeast(1))],
        script_timing_resolver: Some(namespace_unknown_script_timing),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceUnknown),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(NAMESPACE_UNKNOWN_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "upvar",
        // `namespace` + zero-or-more otherVar/myVar PAIRS is a stepped
        // shape, not a flat `at_least` range: Tcl 9.1's own
        // `NamespaceUpvarCmd` (tclNamesp.c) rejects both too few args
        // (`objc < 2`) and an odd count (`objc & 1`) — i.e. a namespace
        // followed by an *incomplete* trailing var is a hard arity error,
        // not merely ignored. Confirmed empirically against real `tclsh`
        // 8.6.14: `namespace upvar ::ns v` (one bare var, no completing
        // myVar) fails with "wrong # args", while `namespace upvar ::ns`
        // (zero pairs) and `namespace upvar ::ns v mv` (one full pair)
        // both succeed. `stepped(1, UNLIMITED, 2)` captures the odd-count-
        // from-1 shape this yields on 8.6+.
        //
        // The Tcl 8.5 manpage's synopsis is `namespace upvar namespace
        // otherVar myVar ?otherVar myVar ...?` ("arranges for ONE OR MORE
        // local variables") — the first pair is mandatory there, so the
        // true 8.5 floor is 3 (odd counts from 3), not 1. From Tcl 8.6 the
        // manpage synopsis changed to `namespace upvar namespace
        // ?otherVar myVar ...?` ("arranges for ZERO OR MORE local
        // variables"): the first pair became optional and the floor
        // dropped to 1. `stepped(1, UNLIMITED, 2)` matches the 8.6/9.0/9.1
        // shape — the loosest common fallback. `NAMESPACE_UPVAR_FORMS` carries
        // the exact per-release split for form-aware consumers.
        arity: Arity::stepped(1, Arity::UNLIMITED, 2),
        detail: "Arrange local variables to refer to namespace variables, as zero or more otherVar/myVar pairs. Tcl 8.5 requires at least one pair; from Tcl 8.6 the namespace argument alone is legal (and a no-op).",
        synopsis: "namespace upvar namespace ?otherVar myVar ...?",
        return_type: Some(TclType::String),
        // The leading word names the namespace the aliased cells live in —
        // relative names root against the current namespace exactly as
        // elsewhere (tclsh 9.0.4 / 8.6.16: inside `namespace eval ::rel`,
        // `namespace upvar kid v alias` binds `::rel::kid::v`).
        arg_roles: &[(0, ArgRole::NamespaceName)],
        // `namespace upvar NS otherVar myVar ?otherVar myVar ...?` — the
        // *local* name of each pair, from index 2 after the subcommand word
        // (issue #1185).
        repeated_args: &[RepeatedArgLayout::strided(ArgRole::VarWrite, 2, 2)],
        subcommand_forms: NAMESPACE_UPVAR_FORMS,
        creates_scope_alias: true,
        surface: Some(SpecSurface::TCL85_PLUS),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceUpvar),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(NAMESPACE_UPVAR_TRANSITIONS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "which",
        // Looks a command up by its spelled name.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        // Exactly one trailing `name`; the two leading flags are declared in
        // `WHICH_OPTIONS`, so the arity check skips them before counting.
        // Verified against tclsh 9.0: 0 args and >1 positional both error.
        arity: Arity::exact(1),
        detail: "Looks up name as either a command or variable.",
        synopsis: "namespace which ?-command? ?-variable? name",
        options: WHICH_OPTIONS,
        arg_role_resolver: Some(namespace_which_arg_roles),
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `namespace`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "namespace",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEVER_INLINE_BODY
            | Traits::HAS_DESTRUCTIVE_OPS
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::InterpState,
                writes: true,
                ..SideEffect::DEFAULT
            },
            // NAMESPACE_STATE.
            SideEffect {
                target: SideEffectTarget::NamespaceState,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Create and manipulate contexts for commands and variables.",
            synopsis: &["namespace subcommand ?arg ...?"],
            snippet: "A namespace is a container for commands and variables, keeping them separate from same-named commands and variables elsewhere in a program. Namespaces nest, using :: to qualify names (::foo::bar::x); the global namespace's real name is the empty string, though :: is accepted everywhere as a synonym. An unqualified command name resolves in the current namespace, then the namespace's command resolution path (Tcl 8.5+, empty by default), then the global namespace, or else the namespace unknown handler. An unqualified variable name resolves the same way through Tcl 8.6 (current namespace, then global); Tcl 9.0 removed the fallback to the global namespace, so an unqualified variable is found only in the current namespace unless declared with `variable` or referenced by a qualified name. Not available in F5 iRules, whose data-plane interpreter strips namespace entirely.",
            source: "Tcl namespace(n)",
            examples: "namespace eval ::counter {\n    variable n 0\n    namespace export bump\n    proc bump {} {\n        variable n\n        incr n\n    }\n}\n::counter::bump\nnamespace eval ::client {\n    namespace import ::counter::bump\n    bump\n}",
            return_value: "",
        }),
        forms: FORMS,
        codegen_hook: Some(crate::hooks::CodegenHookId::Namespace),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NamespaceTransition, NamespaceTransitionTarget, StateTransition, TransitionSubject,
        fold_qualifiers, fold_tail, namespace_delete_state_transitions,
        namespace_path_state_transitions,
    };
    use crate::InvocationArguments;
    use crate::dialects::DialectSet;

    #[test]
    fn namespace_path_keeps_its_tcl_list_operand_whole() {
        let transitions = namespace_path_state_transitions(InvocationArguments::literals(&[
            "path",
            "::pkg {::other child}",
        ]));

        assert!(matches!(
            transitions.facts(),
            [fact]
                if matches!(
                    &fact.transition,
                    StateTransition::Namespace(NamespaceTransition::SetPath {
                        namespace: NamespaceTransitionTarget::Current,
                        path: TransitionSubject::Literal(path),
                    }) if path == "::pkg {::other child}"
                )
        ));
    }

    #[test]
    fn namespace_delete_retains_each_recursive_root() {
        let transitions = namespace_delete_state_transitions(InvocationArguments::literals(&[
            "delete", "::pkg", "::other",
        ]));

        assert!(matches!(
            transitions.facts(),
            [first, second]
                if matches!(
                    &first.transition,
                    StateTransition::Namespace(NamespaceTransition::Delete {
                        namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(name)),
                    }) if name == "::pkg"
                ) && matches!(
                    &second.transition,
                    StateTransition::Namespace(NamespaceTransition::Delete {
                        namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(name)),
                    }) if name == "::other"
                )
        ));
    }

    /// The oracle table, transcribed from tclsh 9.0.4 and 8.6.14 (issue
    /// #1096; the four edge rows the issue tabulates are the last four
    /// here plus `:::`).  Both interpreters produced **byte-identical**
    /// output for every row, so one table pins both.
    ///
    /// ```text
    /// $ tclsh9.0 / tclsh8.6
    /// namespace qualifiers ::a::b::c   -> ::a::b      tail -> c
    /// namespace qualifiers a::b        -> a           tail -> b
    /// namespace qualifiers c           -> {}          tail -> c
    /// namespace qualifiers {}          -> {}          tail -> {}
    /// namespace qualifiers ::          -> {}          tail -> {}
    /// namespace qualifiers :::         -> {}          tail -> {}
    /// namespace qualifiers a:::b       -> a           tail -> b
    /// namespace qualifiers ::a::b::    -> ::a::b      tail -> {}
    /// namespace qualifiers ::x:y       -> {}          tail -> x:y
    /// namespace qualifiers ::foo       -> {}          tail -> foo
    /// namespace qualifiers foo::       -> foo         tail -> {}
    /// namespace qualifiers ::a::b::c:: -> ::a::b::c   tail -> {}
    /// namespace qualifiers a           -> {}          tail -> a
    /// namespace qualifiers :           -> {}          tail -> :
    /// namespace qualifiers ::::        -> {}          tail -> {}
    /// namespace qualifiers x::y::z     -> x::y        tail -> z
    /// namespace qualifiers { a::b }    -> { a}        tail -> {b }
    /// namespace qualifiers {a::b c::d} -> {a::b c}    tail -> d
    /// namespace qualifiers a::         -> a           tail -> {}
    /// namespace qualifiers ::a         -> {}          tail -> a
    /// ```
    const ORACLE: &[(&str, &str, &str)] = &[
        ("::a::b::c", "::a::b", "c"),
        ("a::b", "a", "b"),
        ("c", "", "c"),
        ("", "", ""),
        ("::", "", ""),
        (":::", "", ""),
        ("a:::b", "a", "b"),
        ("::a::b::", "::a::b", ""),
        ("::x:y", "", "x:y"),
        ("::foo", "", "foo"),
        ("foo::", "foo", ""),
        ("::a::b::c::", "::a::b::c", ""),
        ("a", "", "a"),
        (":", "", ":"),
        ("::::", "", ""),
        ("x::y::z", "x::y", "z"),
        ("::ticklecharts::Gauge", "::ticklecharts", "Gauge"),
        (" a::b ", " a", "b "),
        ("a::b c::d", "a::b c", "d"),
        ("a::", "a", ""),
        ("::a", "", "a"),
    ];

    #[test]
    fn qualifiers_and_tail_match_the_oracle_table() {
        for &(input, want_q, want_t) in ORACLE {
            assert_eq!(
                fold_qualifiers(&[input]).as_deref(),
                Some(want_q),
                "namespace qualifiers {input:?}"
            );
            assert_eq!(
                fold_tail(&[input]).as_deref(),
                Some(want_t),
                "namespace tail {input:?}"
            );
        }
    }

    #[test]
    fn folds_are_utf8_safe_on_multibyte_names() {
        // `:` is ASCII, so it never appears inside a multi-byte sequence —
        // every cut point is a char boundary.  Slicing bytes would panic if
        // this were not so.
        assert_eq!(fold_qualifiers(&["é::b"]).as_deref(), Some("é"));
        assert_eq!(fold_tail(&["é::b"]).as_deref(), Some("b"));
        assert_eq!(fold_qualifiers(&["a::é"]).as_deref(), Some("a"));
        assert_eq!(fold_tail(&["a::é"]).as_deref(), Some("é"));
        assert_eq!(fold_qualifiers(&["日本::語"]).as_deref(), Some("日本"));
        assert_eq!(fold_tail(&["日本::語"]).as_deref(), Some("語"));
    }

    #[test]
    fn folds_decline_on_a_wrong_argument_count() {
        // Both subcommands are `arity: exact(1)`; a call tclsh would reject
        // must not fold to a value.
        assert_eq!(fold_qualifiers(&[]), None);
        assert_eq!(fold_tail(&[]), None);
        assert_eq!(fold_qualifiers(&["a", "b"]), None);
        assert_eq!(fold_tail(&["a", "b"]), None);
    }

    #[test]
    fn subcommands_carry_the_folds_through_the_registry() {
        // Registry-level wiring: the optimiser reaches the fold through
        // `SubCommand::run_const_fold`, never by calling the function
        // directly.
        let spec = super::spec();
        let q = spec
            .subcommands
            .iter()
            .find(|s| s.name == "qualifiers")
            .expect("qualifiers subcommand");
        let t = spec
            .subcommands
            .iter()
            .find(|s| s.name == "tail")
            .expect("tail subcommand");
        assert_eq!(
            q.run_const_fold(&["::a::b::c"], Some(tcl_dialect::TclVersion::V9_0))
                .as_deref(),
            Some("::a::b")
        );
        assert_eq!(
            t.run_const_fold(&["::a::b::c"], Some(tcl_dialect::TclVersion::V8_6))
                .as_deref(),
            Some("c")
        );
    }

    #[test]
    fn upvar_forms_own_the_release_specific_zero_pair_arity() {
        let spec = super::spec();
        let upvar = spec
            .subcommands
            .iter()
            .find(|sub| sub.name == "upvar")
            .expect("upvar subcommand");
        let accepts = |dialect, argc| {
            upvar.subcommand_forms.iter().any(|form| {
                form.surface.is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
                    && form.arity.accepts(argc)
            })
        };

        assert!(!accepts(SpecSurface::TCL85, 1));
        assert!(accepts(SpecSurface::TCL85, 3));
        for dialect in [SpecSurface::TCL86, SpecSurface::TCL90, SpecSurface::TCL91] {
            assert!(accepts(dialect, 1));
            assert!(accepts(dialect, 3));
            assert!(!accepts(dialect, 2));
        }
    }

    #[test]
    fn ensemble_management_operations_are_registry_nested_subcommands() {
        // The word after `namespace ensemble` is its own Tcl dispatcher.
        // Keep its exact closed surface and unique-prefix behaviour in the
        // registry so hover/completion/tokens never need a `namespace` branch.
        let spec = super::spec();
        let ensemble = spec
            .subcommands
            .iter()
            .find(|sub| sub.name == "ensemble")
            .expect("ensemble subcommand");
        let names: Vec<_> = ensemble
            .sub_subcommands
            .iter()
            .map(|sub| sub.name)
            .collect();
        assert_eq!(names, ["create", "configure", "exists"]);

        assert_eq!(
            ensemble
                .resolve_sub_subcommand_for_dialect("cre", SpecSurface::TCL90)
                .map(|sub| sub.name),
            Some("create")
        );
        assert_eq!(
            ensemble
                .resolve_sub_subcommand_for_dialect("conf", SpecSurface::TCL85)
                .map(|sub| sub.name),
            Some("configure")
        );
        assert!(
            ensemble
                .resolve_sub_subcommand_for_dialect("e", SpecSurface::TCL84)
                .is_none()
        );
        assert!(ensemble.resolve_sub_subcommand("c").is_none());
    }
}
