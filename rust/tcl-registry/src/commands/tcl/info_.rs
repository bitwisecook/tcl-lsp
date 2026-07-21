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

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "info option ?arg arg ...?",
}];

/// A concise `SubSubCommand` — most `info object`/`info class` operations are
/// available since 8.6 (dialect `None`, inheriting the parent subcommand).
const fn sub(name: &'static str, detail: &'static str, synopsis: &'static str) -> SubSubCommand {
    SubSubCommand {
        name,
        detail,
        synopsis,
        dialects: None,
    }
}

/// A `SubSubCommand` gated to a later dialect (e.g. the 9.0 `TclOO`
/// introspection additions below, TIP 500/524/558).
const fn sub_since(
    name: &'static str,
    detail: &'static str,
    synopsis: &'static str,
    dialects: DialectSet,
) -> SubSubCommand {
    SubSubCommand {
        name,
        detail,
        synopsis,
        dialects: Some(dialects),
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
        "Report the unique creation id of an object.",
        "info object creationid object",
        DialectSet::TCL90_PLUS,
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
        "Test whether an object belongs to a category.",
        "info object isa category object ?arg?",
    ),
    sub(
        "methods",
        "List the methods of an object.",
        "info object methods object ?option ...?",
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
    sub_since(
        "properties",
        "List the declared properties of an object.",
        "info object properties object ?option ...?",
        DialectSet::TCL90_PLUS,
    ),
    sub(
        "variables",
        "List the declared instance variables of an object.",
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
        "Report a class's definition namespace.",
        "info class definitionnamespace class ?kind?",
        DialectSet::TCL90_PLUS,
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
        "info class methods class ?option ...?",
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
    sub_since(
        "properties",
        "List the declared properties of a class.",
        "info class properties class ?option ...?",
        DialectSet::TCL90_PLUS,
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
        "List the declared instance variables of a class.",
        "info class variables class ?-private?",
    ),
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "args",
        traits: Traits::INTROSPECTS_BY_NAME,
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
        dialects: Some(DialectSet::TCL86_PLUS),
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
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cmdtype",
        arity: Arity::exact(1),
        detail: "Returns the type of the command named commandName.",
        synopsis: "info cmdtype commandName",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "commands",
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
        arity: Arity::exact(1),
        detail: "Returns 1 if varName is a constant variable and 0 otherwise.",
        synopsis: "info constant varName",
        pure: true,
        return_type: Some(TclType::Boolean),
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "consts",
        arity: Arity::new(0, 1),
        detail: "Returns the list of constant variables in the current scope.",
        synopsis: "info consts ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "coroutine",
        arity: Arity::exact(0),
        detail: "Returns the name of the current coroutine, or the empty string if there is no current coroutine.",
        synopsis: "info coroutine",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "default",
        traits: Traits::INTROSPECTS_BY_NAME,
        arity: Arity::exact(3),
        detail: "If the parameter has a default value, stores that value in varname and returns 1.",
        synopsis: "info default procname parameter varname",
        return_type: Some(TclType::Boolean),
        // `procname` is an introspected proc (a command reference); `varname`
        // is written.
        arg_roles: &[(0, ArgRole::CommandName), (2, ArgRole::VarWrite)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "errorstack",
        arity: Arity::new(0, 1),
        detail: "Returns a description of the active command at each level from the call stack of the last error.",
        synopsis: "info errorstack ?interp?",
        pure: true,
        return_type: Some(TclType::List),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
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
        arity: Arity::new(0, 1),
        detail: "Returns the depth of the call to info frame itself.",
        synopsis: "info frame ?depth?",
        pure: true,
        return_type: Some(TclType::Dict),
        // Introduced in Tcl 8.5 (TIP 280) — not available in 8.4.
        dialects: Some(DialectSet::TCL85_PLUS),
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
        arity: Arity::new(0, 1),
        detail: "Returns the level this routine was called from.",
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
        detail: "Returns the name of each file loaded in interp by the load command.",
        synopsis: "info loaded ?interp? ?prefix?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "locals",
        traits: Traits::INTROSPECTS_BY_NAME,
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
        dialects: Some(DialectSet::TCL86_PLUS),
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
        arity: Arity::new(0, 1),
        detail: "Returns the names of all visible procedures.",
        synopsis: "info procs ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "script",
        arity: Arity::new(0, 1),
        detail: "Returns the pathname of the innermost script currently being evaluated.",
        synopsis: "info script ?filename?",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
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
        traits: Traits::INTROSPECTS_BY_NAME,
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
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Information about the state of the Tcl interpreter",
            synopsis: &["info option ?arg arg ...?"],
            snippet: "Available commands: info args procname Returns the names of the parameters to the procedure named procname.",
            source: "Tcl man page info.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
