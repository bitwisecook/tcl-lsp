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

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "namespace subcommand ?arg ...?",
    dialects: None,
}];

/// `namespace ensemble create`'s options — the six settable names match
/// this project's own `namespace ensemble` implementations (compare
/// `runtime/rust/src/cmd_namespace.rs`'s `apply_ensemble_option` and
/// `rust/tcl-vm/src/cmd_namespace.rs`'s `ns_ensemble_create`). Every one
/// takes a single value word — `create` (and `configure`'s update form)
/// both parse strict `-option value` pairs, no bare flags. `-namespace`
/// is deliberately excluded: it's a read-only property `configure` can
/// report but neither `create` nor `configure` accepts as a setter
/// (rejected by `apply_ensemble_option`'s `_` arm).
///
/// Dialect gating below is checked directly against the Tcl 8.5, 8.6,
/// 9.0, and 9.1 `namespace.n` manpages (this project's own
/// implementations don't themselves version-gate any option, so they
/// can't be the source of truth here): `-command`, `-map`, `-prefixes`,
/// `-subcommands`, and `-unknown` are present from 8.5 (when `namespace
/// ensemble` itself was introduced) onward and so inherit the
/// subcommand's own `TCL85_PLUS` gate. `-parameters` is the one
/// exception — it is absent from the Tcl 8.5 ENSEMBLE OPTIONS list
/// (missing from both the option table and the `-map`/`-prefixes`/
/// `-subcommands`/`-unknown`/`-command`/`-namespace` enumeration) and
/// first appears in the Tcl 8.6 manpage, so it carries its own,
/// narrower `TCL86_PLUS` gate.
static ENSEMBLE_CREATE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        value: OptionValue::value("name"),
        detail: "Name of the ensemble's dispatch command (default: the fully-qualified name of the invoking namespace). Write-only, and valid only with create.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-map",
        value: OptionValue::value("dict"),
        detail: "Maps subcommand names to target command-prefix lists, similar to interp alias (default: empty, meaning each subcommand maps to the identically-named command in the linked namespace).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-parameters",
        value: OptionValue::value("list"),
        detail: "Named arguments inserted between the ensemble command and the subcommand, used when generating error messages (default: none).",
        dialects: Some(DialectSet::TCL86_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-prefixes",
        value: OptionValue::value("boolean"),
        detail: "Whether unambiguous subcommand prefixes are accepted (default: on).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-subcommands",
        value: OptionValue::value("list"),
        detail: "Explicit list of valid subcommand names (default: empty, meaning the -map keys or the linked namespace's exported commands).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-unknown",
        value: OptionValue::command_prefix("prefix"),
        detail: "Command prefix invoked, with the ensemble's own invocation words appended, when a subcommand is not recognised (default: none, which raises a standard \"unknown subcommand\" error).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
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
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::flag(),
        detail: "Resolve name as a variable.",
        dialects: None,
        aliases: &[],
        min_version: None,
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

/// `namespace export`'s only flag — present unchanged in the synopsis of
/// every fetched version (8.4 through 9.1).
static EXPORT_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-clear",
    value: OptionValue::flag(),
    detail: "Reset the namespace's export pattern list to empty before appending the given patterns.",
    dialects: None,
    aliases: &[],
    min_version: None,
}];

/// `namespace import`'s only flag — present unchanged in the synopsis of
/// every fetched version (8.4 through 9.1).
static IMPORT_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-force",
    value: OptionValue::flag(),
    detail: "Silently overwrite an existing command instead of erroring on conflict.",
    dialects: None,
    aliases: &[],
    min_version: None,
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ensemble",
        arity: Arity::at_least(1),
        detail: "Creates and manipulates a command ensemble.",
        synopsis: "namespace ensemble subcommand ?arg ...?",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL85_PLUS),
        // Shared by `create`/`configure` (see `ENSEMBLE_CREATE_OPTIONS`'s
        // own doc comment) — `ensemble`'s own dispatch (`create` /
        // `configure` / `exists`) isn't modelled as nested subcommands, so
        // this covers the whole `namespace ensemble …` surface rather than
        // just `create`'s.
        options: ENSEMBLE_CREATE_OPTIONS,
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
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceEnsemble),
        // An ensemble publishes its namespace's commands under a single
        // dispatching command name (and `-map` can redirect a subcommand
        // to an arbitrary prefix), so the mapped commands acquire callers
        // that need not appear in this file at all.
        traits: Traits::EXPORTS_COMMAND,
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
        // does not contain.
        traits: Traits::EXPORTS_COMMAND,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "import",
        arity: Arity::any(),
        // The bare (no-pattern, no-flag) query form — "returns the list of
        // commands in the current namespace that have been imported from
        // other namespaces" — is documented starting with the Tcl 8.5
        // manpage; the Tcl 8.4 manpage only describes the importing
        // behaviour, with no query-form return value stated for a
        // zero-argument call. `import` itself has no dialect gate (it is
        // present in 8.4 too), so this is a behavioural note rather than a
        // `dialects:` restriction — see the module-level task guidance on
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
        dialects: Some(DialectSet::TCL85_PLUS),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespacePath),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "qualifiers",
        arity: Arity::exact(1),
        detail: "Returns any leading namespace qualifiers for string.",
        synopsis: "namespace qualifiers string",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tail",
        arity: Arity::exact(1),
        detail: "Returns the simple name at the end of a qualified string.",
        synopsis: "namespace tail string",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unknown",
        arity: Arity::new(0, 1),
        detail: "Sets or returns the unknown command handler for the current namespace.",
        synopsis: "namespace unknown ?script?",
        return_type: Some(TclType::String),
        // Added in 8.5 (`NamespaceUnknownCmd`, tclNamesp.c), like the sibling
        // `namespace path`; 8.4's `namespace` ensemble has no `unknown`
        // subcommand.  Gate it the same as `path` so an 8.4 document flags it.
        dialects: Some(DialectSet::TCL85_PLUS),
        // The optional handler (index 0 after `unknown` → arg 1) is a command
        // prefix invoked with the unknown command name + its args appended
        // (variadic ⇒ AtLeast(1)). The zero-arg query form has no prefix.
        command_prefixes: &[(0, AppendedArity::AtLeast(1))],
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceUnknown),
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
        // shape — the loosest of the two that never rejects a valid 8.6+
        // call; Tcl 8.5's stricter 3-word floor is documented in `detail`
        // below rather than modelled structurally, since `Arity` has no
        // per-dialect variant and the gap only matters for the
        // essentially-unused zero-pair call shape.
        arity: Arity::stepped(1, Arity::UNLIMITED, 2),
        detail: "Arrange local variables to refer to namespace variables, as zero or more otherVar/myVar pairs. Tcl 8.5 requires at least one pair; from Tcl 8.6 the namespace argument alone is legal (and a no-op).",
        synopsis: "namespace upvar namespace ?otherVar myVar ...?",
        return_type: Some(TclType::String),
        // The leading word names the namespace the aliased cells live in —
        // relative names root against the current namespace exactly as
        // elsewhere (tclsh 9.0.4 / 8.6.16: inside `namespace eval ::rel`,
        // `namespace upvar kid v alias` binds `::rel::kid::v`).
        arg_roles: &[(0, ArgRole::NamespaceName)],
        creates_scope_alias: true,
        dialects: Some(DialectSet::TCL85_PLUS),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceUpvar),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "which",
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
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEVER_INLINE_BODY
            | Traits::HAS_DESTRUCTIVE_OPS
            | Traits::DYNAMIC_EVAL_BODY
            | Traits::WASM_EMITS_NOTHING,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::InterpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            // NAMESPACE_STATE.
            SideEffect {
                target: SideEffectTarget::NamespaceState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
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
