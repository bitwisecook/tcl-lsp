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
}];

/// `namespace ensemble create`'s options — verified against this project's
/// own `namespace ensemble` implementation
/// (`runtime/rust/src/cmd_namespace.rs`'s `ens_create` /
/// `apply_ensemble_option`), whose "bad option" error text enumerates
/// exactly these six. Every one takes a single value word — `create` (and
/// `configure`'s update form) both parse strict `-option value` pairs, no
/// bare flags. `-namespace` is deliberately excluded: it's a read-only
/// property `configure` can report but neither `create` nor `configure`
/// accepts as a setter (rejected by `apply_ensemble_option`'s `_` arm).
static ENSEMBLE_CREATE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        value: OptionValue::value("name"),
        detail: "Name of the ensemble's dispatch command (default: the namespace's own name).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-map",
        value: OptionValue::value("dict"),
        detail: "Maps subcommand names to target command prefixes.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-parameters",
        value: OptionValue::value("list"),
        detail: "Parameter names inserted between the ensemble command and the subcommand.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-prefixes",
        value: OptionValue::value("boolean"),
        detail: "Whether unambiguous subcommand prefixes are accepted.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-subcommands",
        value: OptionValue::value("list"),
        detail: "Explicit list of valid subcommand names.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-unknown",
        value: OptionValue::value("prefix"),
        detail: "Command prefix invoked for an unrecognised subcommand.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

/// `namespace which ?-command? ?-variable? name` — the two leading flags select
/// what to resolve `name` as. They are bare (value-less) flags; declaring them
/// lets the arity check skip them so `exact(1)` counts only the trailing `name`
/// (catching `namespace which foo bar`), and lights up their completion/hover.
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

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "children",
        arity: Arity::new(0, 2),
        detail: "Returns a list of all child namespaces.",
        synopsis: "namespace children ?namespace? ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "code",
        arity: Arity::exact(1),
        detail: "Captures the current namespace context for later execution.",
        synopsis: "namespace code script",
        pure: true,
        return_type: Some(TclType::String),
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
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceEnsemble),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::at_least(2),
        detail: "Evaluate a script in a namespace context.",
        synopsis: "namespace eval namespace arg ?arg ...?",
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::NamespaceEval),
        return_type: Some(TclType::String),
        // The body evaluates in the *namespace* frame, not the caller's, so
        // SSA must not recover its `$var` reads as caller-local reads (else a
        // proc param read only inside the body looks used).  The body's
        // `$var` reads are excluded from caller-local read recovery.
        body_kind: BodyKind::Structural,
        // Runs an arbitrary script that can touch namespace/global state —
        // dynamic-dispatch consumers (memory-SSA clobber classification)
        // key off this.
        traits: Traits::EVALUATES_CODE,
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "export",
        arity: Arity::any(),
        detail: "Specifies which commands are exported from a namespace.",
        synopsis: "namespace export ?-clear? ?pattern pattern ...?",
        return_type: Some(TclType::List),
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "import",
        arity: Arity::any(),
        detail: "Imports commands into a namespace.",
        synopsis: "namespace import ?-force? ?pattern pattern ...?",
        return_type: Some(TclType::List),
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceImport),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inscope",
        arity: Arity::at_least(2),
        detail: "Executes a script in the context of the specified namespace.",
        synopsis: "namespace inscope namespace script ?arg ...?",
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        return_type: Some(TclType::String),
        // Like `eval`, the script runs in the namespace frame.
        body_kind: BodyKind::Structural,
        traits: Traits::EVALUATES_CODE,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "origin",
        arity: Arity::exact(1),
        detail: "Returns the fully-qualified name of the original command.",
        synopsis: "namespace origin command",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "parent",
        arity: Arity::new(0, 1),
        detail: "Returns the fully-qualified name of the parent namespace.",
        synopsis: "namespace parent ?namespace?",
        pure: true,
        return_type: Some(TclType::String),
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
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        // The optional handler (index 0 after `unknown` → arg 1) is a command
        // prefix invoked with the unknown command name + its args appended
        // (variadic ⇒ AtLeast(1)). The zero-arg query form has no prefix.
        command_prefixes: &[(0, AppendedArity::AtLeast(1))],
        analyser_hook: Some(crate::hooks::AnalyserHookId::NamespaceUnknown),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "upvar",
        arity: Arity::at_least(1),
        detail: "Arrange local variables to refer to namespace variables.",
        synopsis: "namespace upvar namespace ?otherVar myVar ...?",
        return_type: Some(TclType::String),
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
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `namespace`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "namespace",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
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
            },
            // NAMESPACE_STATE.
            SideEffect {
                target: SideEffectTarget::NamespaceState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
            },
        ],
        hover: Some(HoverSnippet {
            summary: "create and manipulate contexts for commands and variables",
            synopsis: &["namespace subcommand ?arg ...?"],
            snippet: "The namespace command lets you create, access, and destroy separate contexts for commands and variables.",
            source: "Tcl man page namespace.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        codegen_hook: Some(crate::hooks::CodegenHookId::Namespace),
        ..CommandSpec::DEFAULT
    }
}
