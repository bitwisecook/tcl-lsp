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

//! `info` — information about the state of the Tcl interpreter.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
use tcl_dialect::TclVersion;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "info option ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

/// A concise `SubSubCommand` — most `info object`/`info class` operations are
/// available since 8.6 (dialect `None`, inheriting the parent subcommand).
///
/// Every `9.0`-gated fact below (`sub_since` call sites) was cross-checked
/// against the Tcl 9.1 beta manpage as well: `info.n` in 9.1 is byte-for-byte
/// identical to 9.0 apart from the version banner, so a `TCL90_PLUS` gate is
/// exact for both releases — there is no 9.1-only delta to model separately.
const fn sub(name: &'static str, detail: &'static str, synopsis: &'static str) -> SubSubCommand {
    SubSubCommand {
        name,
        detail,
        synopsis,
        ..SubSubCommand::DEFAULT
    }
}

/// A `SubSubCommand` gated to a later dialect (e.g. the 9.0 `TclOO`
/// introspection additions below, TIP 500/524/558).
///
/// `since` states the same fact on the *version* axis that `dialects` states
/// on the dialect-bit axis: `info` is a core command with no owning package,
/// so its lifecycle axis is the Tcl core release the file targets. The two
/// are kept as separate arguments rather than derived from one another
/// because a dialect set is a membership mask, not an ordered release.
const fn sub_since(
    name: &'static str,
    detail: &'static str,
    synopsis: &'static str,
    surface: &'static [SpecSurface],
    since: &'static str,
) -> SubSubCommand {
    SubSubCommand {
        name,
        detail,
        synopsis,
        surface: Some(surface),
        lifecycle: Lifecycle::introduced_in(since),
        ..SubSubCommand::DEFAULT
    }
}

const INFO_PROPERTIES_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-all",
        detail: "Include inherited properties.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-readable",
        detail: "List readable properties (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-writable",
        detail: "List writable properties.",
        ..OptionSpec::DEFAULT
    },
];

const fn properties_sub(detail: &'static str, synopsis: &'static str) -> SubSubCommand {
    SubSubCommand {
        name: "properties",
        detail,
        synopsis,
        options: Some(INFO_PROPERTIES_OPTIONS),
        surface: Some(SpecSurface::TCL90_PLUS),
        lifecycle: Lifecycle::introduced_in("9.0"),
    }
}

/// Second-level subcommands of `info object` (the OBJECT INTROSPECTION
/// operations from the `info` man page). Base set is Tcl 8.6; `creationid`
/// is 9.0 (TIP 500) and `properties` is 9.0 (TIP 558).
const INFO_OBJECT_SUBS: &[SubSubCommand] = &[
    sub(
        "call",
        "Report the method-call chain for a method.",
        "info object call object methodName",
    ),
    sub(
        "class",
        "Report the class of an object (or test membership).",
        "info object class object ?className?",
    ),
    sub_since(
        "creationid",
        "Report the object's unique creation id, fixed for its lifetime.",
        "info object creationid object",
        SpecSurface::TCL90_PLUS,
        "9.0",
    ),
    sub(
        "definition",
        "Report how a method was defined.",
        "info object definition object methodName",
    ),
    sub(
        "filters",
        "List the filter methods of an object.",
        "info object filters object",
    ),
    sub(
        "forward",
        "Report the target of a forwarded method.",
        "info object forward object methodName",
    ),
    sub(
        "isa",
        "Test whether an object belongs to a category: class, metaclass, mixin, object, or typeof.",
        "info object isa category object ?arg?",
    ),
    sub(
        "methods",
        "List the methods of an object.",
        "info object methods object ?option...?",
    ),
    sub(
        "methodtype",
        "Report the type of a method.",
        "info object methodtype object methodName",
    ),
    sub(
        "mixins",
        "List the classes mixed into an object.",
        "info object mixins object",
    ),
    sub(
        "namespace",
        "Report the private namespace of an object.",
        "info object namespace object",
    ),
    properties_sub(
        "List the declared properties of an object.",
        "info object properties object ?options...?",
    ),
    sub(
        "variables",
        "List the declared instance variables of an object. Tcl 9.0+ accepts an optional -private flag to list private variables instead.",
        "info object variables object",
    ),
    sub(
        "vars",
        "List the visible variables in an object's namespace.",
        "info object vars object ?pattern?",
    ),
];

/// Second-level subcommands of `info class` (the CLASS INTROSPECTION
/// operations from the `info` man page). Base set is Tcl 8.6;
/// `definitionnamespace` is 9.0 (TIP 524) and `properties` is 9.0 (TIP 558).
const INFO_CLASS_SUBS: &[SubSubCommand] = &[
    sub(
        "call",
        "Report the method-call chain for a class method.",
        "info class call class methodName",
    ),
    sub(
        "constructor",
        "Report the definition of a class constructor.",
        "info class constructor class",
    ),
    sub(
        "definition",
        "Report how a class method was defined.",
        "info class definition class methodName",
    ),
    sub_since(
        "definitionnamespace",
        "Report the definition namespace used for kind definitions of the class: -class (the default) or -instance.",
        "info class definitionnamespace class ?kind?",
        SpecSurface::TCL90_PLUS,
        "9.0",
    ),
    sub(
        "destructor",
        "Report the definition of a class destructor.",
        "info class destructor class",
    ),
    sub(
        "filters",
        "List the filter methods of a class.",
        "info class filters class",
    ),
    sub(
        "forward",
        "Report the target of a forwarded class method.",
        "info class forward class methodName",
    ),
    sub(
        "instances",
        "List the instances of a class.",
        "info class instances class ?pattern?",
    ),
    sub(
        "methods",
        "List the methods of a class.",
        "info class methods class ?options...?",
    ),
    sub(
        "methodtype",
        "Report the type of a class method.",
        "info class methodtype class methodName",
    ),
    sub(
        "mixins",
        "List the classes mixed into a class.",
        "info class mixins class",
    ),
    properties_sub(
        "List the declared properties of a class.",
        "info class properties class ?options...?",
    ),
    sub(
        "subclasses",
        "List the subclasses of a class.",
        "info class subclasses class ?pattern?",
    ),
    sub(
        "superclasses",
        "List the superclasses of a class.",
        "info class superclasses class",
    ),
    sub(
        "variables",
        "List the declared instance variables of a class. Tcl 9.0+ accepts an optional -private flag to list private variables instead.",
        "info class variables class",
    ),
];

/// Which `TclOO` second-level `info` ensemble is being dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoOoEnsembleKind {
    /// `info object subcommand ...`
    Object,
    /// `info class subcommand ...`
    Class,
}

/// One option accepted by Tcl 9.0's `info object properties` and
/// `info class properties` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoOoPropertiesOption {
    /// Include inherited properties.
    All,
    /// Select readable properties.
    Readable,
    /// Select writable properties.
    Writable,
}

/// Resolve an `info ... properties` option through the option rows attached
/// to the registry's `properties` operation.
///
/// # Errors
/// Tcl's byte-exact bad/ambiguous option message.
pub fn resolve_info_oo_properties_option(word: &[u8]) -> Result<InfoOoPropertiesOption, Vec<u8>> {
    let names: Vec<&str> = INFO_PROPERTIES_OPTIONS
        .iter()
        .map(|option| option.name)
        .collect();
    match tcl_cmd_core::prefix::OptionTable::abbreviating("option", &names).index_of(word)? {
        0 => Ok(InfoOoPropertiesOption::All),
        1 => Ok(InfoOoPropertiesOption::Readable),
        2 => Ok(InfoOoPropertiesOption::Writable),
        _ => unreachable!("the registry declares exactly three info properties options"),
    }
}

impl InfoOoEnsembleKind {
    const fn parent_name(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Class => "class",
        }
    }
}

/// The registry-derived, release-filtered operation table for one `TclOO`
/// `info` ensemble.
///
/// The registry owns which operations exist; `tcl-cmd-core::ensemble` owns
/// exact/unique-prefix resolution and the ensemble-specific `, or` rendering.
/// Runtime consumers therefore receive one ready-to-dispatch table and never
/// copy either the operation names or the miss-message list.
#[derive(Debug, Clone)]
pub struct InfoOoSubcommands {
    names: Vec<&'static str>,
}

impl InfoOoSubcommands {
    /// Canonical operation names in Tcl's listing order.
    #[must_use]
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

    /// Resolve an exact name or unique prefix, returning the canonical name.
    ///
    /// # Errors
    /// The Tcl ensemble miss message, including the release-filtered choices.
    pub fn resolve(&self, word: &[u8]) -> Result<&'static str, Vec<u8>> {
        tcl_cmd_core::ensemble::resolve_subcommand(&self.names, word, true)
            .map(|index| self.names[index])
            .ok_or_else(|| {
                tcl_cmd_core::ensemble::unknown_subcommand_message(&self.names, word, true, b"")
            })
    }
}

/// Build the authoritative operation table for `info object` or `info class`
/// at `version`.
///
/// This projects the same [`SubSubCommand`] rows used by diagnostics,
/// completion, and hover. In particular, Tcl 9.0's `creationid`,
/// `definitionnamespace`, and `properties` rows disappear on an 8.6 target
/// before prefix uniqueness or error rendering is decided.
#[must_use]
pub fn info_oo_subcommands(kind: InfoOoEnsembleKind, version: TclVersion) -> InfoOoSubcommands {
    let dialect =
        Some(crate::model::static_document_context_for(version.dialect_name()).authoring_query());
    let info = spec();
    let parent = info
        .subcommands
        .iter()
        .find(|subcommand| subcommand.name == kind.parent_name())
        .expect("the info spec declares both TclOO ensembles");
    InfoOoSubcommands {
        names: parent
            .available_sub_subcommands(dialect, None)
            .into_iter()
            .map(|subcommand| subcommand.name)
            .collect(),
    }
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "args",
        // Reflects another proc's parameter names, looked up by the proc's
        // spelled name — observable identity for both symbol kinds.
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(1),
        detail: "Returns the names of the parameters to the procedure named procname.",
        synopsis: "info args procname",
        pure: true,
        return_type: Some(TclType::List),
        // `procname` names a proc introspected (not called), so it is a
        // command reference navigation follows.
        arg_roles: &[(0, ArgRole::CommandName)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "body",
        // Reflects a proc's source (including its local-variable spellings)
        // by the proc's spelled name.
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(1),
        detail: "Returns the body of the procedure named procname.",
        synopsis: "info body procname",
        pure: true,
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::CommandName)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "class",
        arity: Arity::at_least(2),
        detail: "Returns information about the class.",
        synopsis: "info class subcommand class ?arg ...?",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL86_PLUS),
        // `info class` is itself an ensemble: the word after `class` selects a
        // CLASS INTROSPECTION operation (issue #798).
        sub_subcommands: INFO_CLASS_SUBS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cmdcount",
        arity: Arity::exact(0),
        detail: "Returns the total number of commands evaluated in this interpreter.",
        synopsis: "info cmdcount",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cmdtype",
        // Introspects a command by its spelled name.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::exact(1),
        detail: "Returns the type of the command named commandName: alias, coroutine, ensemble, import, native, object, privateObject, proc, interp, or zlibStream.",
        synopsis: "info cmdtype commandName",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL90_PLUS),
        // `commandName` is an introspected command (a command reference),
        // matching `info args`/`info body`'s treatment of `procname`.
        arg_roles: &[(0, ArgRole::CommandName)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "commands",
        // Enumerates command names — reflection over the command table.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::new(0, 1),
        detail: "Returns the names of all commands visible in the current namespace.",
        synopsis: "info commands ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        // The optional argument is a glob *pattern*, but the common exact
        // spelling (`info commands foo`) probes one specific command — a
        // navigable reference that asserts nothing about existence (an
        // absent name returns an empty list).  The analyser's probe
        // recorder abstains on any word with glob metacharacters, so a
        // real pattern contributes no reference (issue #945 fault 9).
        arg_roles: &[(0, ArgRole::CommandNameProbe)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "complete",
        arity: Arity::exact(1),
        detail: "Returns 1 if command is a complete command, and 0 otherwise.",
        synopsis: "info complete command",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "constant",
        traits: Traits::INTROSPECTS_BY_NAME,
        arity: Arity::exact(1),
        detail: "Returns 1 if varName is a constant variable and 0 otherwise.",
        synopsis: "info constant varName",
        pure: true,
        return_type: Some(TclType::Boolean),
        surface: Some(SpecSurface::TCL90_PLUS),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "consts",
        arity: Arity::new(0, 1),
        detail: "Returns the list of constant variables in the current scope.",
        synopsis: "info consts ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        surface: Some(SpecSurface::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "coroutine",
        traits: Traits::CURRENT_FRAME_INTROSPECTION,
        arity: Arity::exact(0),
        detail: "Returns the name of the current coroutine, or the empty string if there is no current coroutine.",
        synopsis: "info coroutine",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "default",
        // Reflects a named proc's parameter defaults by the proc's spelled
        // name — observable identity for both symbol kinds.
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(3),
        detail: "If the parameter has a default value, stores that value in varname and returns 1; otherwise returns 0.",
        synopsis: "info default procname parameter varname",
        return_type: Some(TclType::Boolean),
        // `procname` is an introspected proc (a command reference); `varname`
        // is written.
        arg_roles: &[(0, ArgRole::CommandName), (2, ArgRole::VarWrite)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "errorstack",
        traits: Traits::CURRENT_FRAME_INTROSPECTION,
        arity: Arity::new(0, 1),
        detail: "Returns an even-sized list of CALL/UP/INNER tokens and parameters describing the active command at each level from the call stack of the last error. Also available as the -errorstack entry of a 3-argument catch's options dictionary.",
        synopsis: "info errorstack ?interp?",
        pure: true,
        return_type: Some(TclType::List),
        surface: Some(SpecSurface::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::InfoExists)),
        traits: Traits::INTROSPECTS_BY_NAME,
        arity: Arity::exact(1),
        detail: "Returns 1 if a variable named varName is visible and has been defined, and 0 otherwise.",
        synopsis: "info exists varName",
        pure: true,
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        inline_codegen_hook: Some(InlineCodegenHookId::InfoExists),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "frame",
        // Reflects the active command words (including proc names) of any
        // stack frame.
        traits: Traits::REFLECTS_COMMAND_NAMES.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the depth of the call to info frame itself when depth is omitted; otherwise returns a dictionary describing the active command at that depth (keys: type — one of source/proc/eval/precompiled — plus line, file, cmd, proc, lambda, level as applicable). Reports every stack frame, including eval/uplevel/source frames that info level does not see.",
        synopsis: "info frame ?depth?",
        pure: true,
        return_type: Some(TclType::Dict),
        // Introduced in Tcl 8.5 (TIP 280) — not available in 8.4.
        surface: Some(SpecSurface::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "functions",
        arity: Arity::new(0, 1),
        detail: "Returns a list of all the math functions currently defined.",
        synopsis: "info functions ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "globals",
        // Enumerates global variable names — the global-scope counterpart
        // of `info vars` / `info locals`.
        traits: Traits::INTROSPECTS_BY_NAME,
        arity: Arity::new(0, 1),
        detail: "Returns a list of all the names of currently-defined global variables.",
        synopsis: "info globals ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hostname",
        arity: Arity::exact(0),
        detail: "Returns the name of the current host.",
        synopsis: "info hostname",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "level",
        // `info level N` reflects the full command (name + arguments) at
        // that level, so proc names are observable data.
        traits: Traits::REFLECTS_COMMAND_NAMES.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the current stack level (0 at top level) when level is omitted; otherwise returns the complete command (name and arguments, as a list) active at that level. A positive level is absolute (1 = outermost active procedure); zero or a negative level is relative to the current one.",
        synopsis: "info level ?level?",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "library",
        arity: Arity::exact(0),
        detail: "Returns the name of the library directory in which standard Tcl scripts are stored.",
        synopsis: "info library",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "loaded",
        arity: Arity::new(0, 2),
        detail: "Returns the name of each file loaded in interp by the load command, paired with the package prefix it was loaded under. From Tcl 9.0, an optional trailing prefix argument restricts the results to that prefix; Tcl 8.4-8.6 accept only the interp argument.",
        synopsis: "info loaded ?interp? ?prefix?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "locals",
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the name of each local variable matching pattern.",
        synopsis: "info locals ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nameofexecutable",
        arity: Arity::exact(0),
        detail: "Returns the absolute pathname of the program for the current interpreter.",
        synopsis: "info nameofexecutable",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "object",
        arity: Arity::at_least(2),
        detail: "Returns information about the object.",
        synopsis: "info object subcommand object ?arg ...?",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL86_PLUS),
        // `info object` is itself an ensemble: the word after `object` selects
        // an OBJECT INTROSPECTION operation (issue #798).
        sub_subcommands: INFO_OBJECT_SUBS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "patchlevel",
        arity: Arity::exact(0),
        detail: "Returns the value of the global variable tcl_patchLevel.",
        synopsis: "info patchlevel",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "procs",
        // Enumerates procedure names — reflection over the command table.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::new(0, 1),
        detail: "Returns the names of all visible procedures.",
        synopsis: "info procs ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        // Same probe semantics as `info commands` above: an exact spelling
        // (`info procs helper`) is a navigable reference to an existing
        // proc, a glob pattern contributes none.
        arg_roles: &[(0, ArgRole::CommandNameProbe)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "script",
        arity: Arity::new(0, 1),
        detail: "Returns the pathname of the innermost script currently being evaluated, or the empty string if none. With filename, overrides the return value of this command for the remainder of the active invocation — useful in virtual filesystem applications.",
        synopsis: "info script ?filename?",
        pure: true,
        mutator: true,
        return_type: Some(TclType::String),
        returns_path: true,
        // Read-only in its common bare form; the optional `filename`
        // argument writes interpreter-level state (the active script-path
        // override), unchanged across 8.4-9.1. `mutator: true` is required
        // alongside `pure: true` here (mirrors the getter/setter pattern
        // used e.g. by `DNS::header cd ?value?`) — without it,
        // `tcl_compiler::side_effects::classify_side_effects`'s
        // `sub.pure && !sub.mutator` short-circuit returns an empty,
        // effect-free classification for *every* call before the
        // `side_effects` below is ever consulted, silently making
        // `info script $path` look pure/dead-code-eliminable even though
        // it exists only for its side effect.
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sharedlibextension",
        arity: Arity::exact(0),
        detail: "Returns the extension used on this platform for shared libraries.",
        synopsis: "info sharedlibextension",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tclversion",
        arity: Arity::exact(0),
        detail: "Returns the major and minor version of the Tcl library.",
        synopsis: "info tclversion",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vars",
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the names of all visible variables.",
        synopsis: "info vars ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `info`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "info",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Information about the state of the Tcl interpreter",
            synopsis: &["info option ?arg arg ...?"],
            snippet: "A large introspection ensemble; option names may be abbreviated to any unambiguous prefix. Covers procedure metadata (args, body, default, procs), variable metadata (exists, vars, globals, locals), and command/interpreter/environment metadata (commands, cmdcount, complete, level, loaded, hostname, library, nameofexecutable, patchlevel, script, sharedlibextension, tclversion). Tcl 8.5 adds stack-frame introspection (frame). Tcl 8.6 adds coroutine, errorstack, and the TclOO info class/info object introspection subcommands. Tcl 9.0 adds cmdtype, constant, consts, an optional prefix argument to loaded, and extends the TclOO introspection with creationid, definitionnamespace, and properties. Tcl 9.1 makes no further changes to info.",
            source: "Tcl info(n)",
            examples: "if {[info exists myVar]} {\n    puts $myVar\n}\nforeach p [info procs helper*] {\n    puts \"found $p\"\n}\nputs \"Tcl [info tclversion] (patchlevel [info patchlevel])\"",
            return_value: "Varies by subcommand: most return a string, list, or boolean. info level and info frame return an integer when called with no argument, and a list or dictionary respectively when given one. See each subcommand's own description for its exact shape.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJECT_86: &[&str] = &[
        "call",
        "class",
        "definition",
        "filters",
        "forward",
        "isa",
        "methods",
        "methodtype",
        "mixins",
        "namespace",
        "variables",
        "vars",
    ];
    const OBJECT_90: &[&str] = &[
        "call",
        "class",
        "creationid",
        "definition",
        "filters",
        "forward",
        "isa",
        "methods",
        "methodtype",
        "mixins",
        "namespace",
        "properties",
        "variables",
        "vars",
    ];
    const CLASS_86: &[&str] = &[
        "call",
        "constructor",
        "definition",
        "destructor",
        "filters",
        "forward",
        "instances",
        "methods",
        "methodtype",
        "mixins",
        "subclasses",
        "superclasses",
        "variables",
    ];
    const CLASS_90: &[&str] = &[
        "call",
        "constructor",
        "definition",
        "definitionnamespace",
        "destructor",
        "filters",
        "forward",
        "instances",
        "methods",
        "methodtype",
        "mixins",
        "properties",
        "subclasses",
        "superclasses",
        "variables",
    ];

    #[test]
    fn tcloo_info_tables_are_release_filtered_from_registry_rows() {
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Object, TclVersion::V8_6).names(),
            OBJECT_86
        );
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Object, TclVersion::V9_0).names(),
            OBJECT_90
        );
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V8_6).names(),
            CLASS_86
        );
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V9_0).names(),
            CLASS_90
        );
    }

    #[test]
    fn tcloo_info_resolution_and_choices_match_tcl_ensembles() {
        let object = info_oo_subcommands(InfoOoEnsembleKind::Object, TclVersion::V9_0);
        assert_eq!(object.resolve(b"cl"), Ok("class"));
        assert_eq!(
            object.resolve(b"bogus").unwrap_err(),
            b"unknown or ambiguous subcommand \"bogus\": must be call, class, creationid, definition, filters, forward, isa, methods, methodtype, mixins, namespace, properties, variables, or vars"
        );

        let class_86 = info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V8_6);
        let class_90 = info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V9_0);
        assert_eq!(class_86.resolve(b"def"), Ok("definition"));
        assert_eq!(
            class_90.resolve(b"def").unwrap_err(),
            b"unknown or ambiguous subcommand \"def\": must be call, constructor, definition, definitionnamespace, destructor, filters, forward, instances, methods, methodtype, mixins, properties, subclasses, superclasses, or variables"
        );
    }

    #[test]
    fn tcloo_info_properties_options_use_registry_rows_and_shared_prefixes() {
        assert_eq!(
            resolve_info_oo_properties_option(b"-a"),
            Ok(InfoOoPropertiesOption::All)
        );
        assert_eq!(
            resolve_info_oo_properties_option(b"-r"),
            Ok(InfoOoPropertiesOption::Readable)
        );
        assert_eq!(
            resolve_info_oo_properties_option(b"-").unwrap_err(),
            b"ambiguous option \"-\": must be -all, -readable, or -writable"
        );
        assert_eq!(
            resolve_info_oo_properties_option(b"-bogus").unwrap_err(),
            b"bad option \"-bogus\": must be -all, -readable, or -writable"
        );

        for subs in [INFO_OBJECT_SUBS, INFO_CLASS_SUBS] {
            let properties = subs
                .iter()
                .find(|subcommand| subcommand.name == "properties")
                .expect("properties operation");
            assert_eq!(
                properties
                    .options
                    .expect("registry properties options")
                    .iter()
                    .map(|option| option.name)
                    .collect::<Vec<_>>(),
                ["-all", "-readable", "-writable"]
            );
        }
    }
}
