# Scaffolded from info.n -- refine and commit
"""info -- Information about the state of the Tcl interpreter."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page info.n"


_av = make_av(_SOURCE)


@register
class InfoCommand(CommandDef):
    name = "info"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="info",
            hover=HoverSnippet(
                summary="Information about the state of the Tcl interpreter",
                synopsis=("info option ?arg arg ...?",),
                snippet="Available commands: info args procname Returns the names of the parameters to the procedure named procname.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="info option ?arg arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "args",
                                "Returns the names of the parameters to the procedure named procname.",
                                "info args procname",
                            ),
                            _av(
                                "body",
                                "Returns the body of the procedure named procname.",
                                "info body procname",
                            ),
                            _av(
                                "cmdcount",
                                "Returns the total number of commands evaluated in this interpreter.",
                                "info cmdcount",
                            ),
                            _av(
                                "commands",
                                "Returns the names of all commands visible in the current namespace.",
                                "info commands ?pattern?",
                            ),
                            _av(
                                "complete",
                                "Returns 1 if command is a complete command, and 0 otherwise.",
                                "info complete command",
                            ),
                            _av(
                                "default",
                                "If the parameter parameter for the procedure named procname has a default value, stores that value in varname and returns 1.",
                                "info default procname parameter varname",
                            ),
                            _av(
                                "exists",
                                "Returns 1 if a variable named varName is visible and has been defined, and 0 otherwise.",
                                "info exists varName",
                            ),
                            _av(
                                "frame",
                                "Returns the depth of the call to info frame itself.",
                                "info frame ?depth?",
                            ),
                            _av(
                                "functions",
                                "If pattern is not given, returns a list of all the math functions currently defined.",
                                "info functions ?pattern?",
                            ),
                            _av(
                                "globals",
                                "If pattern is not given, returns a list of all the names of currently-defined global variables.",
                                "info globals ?pattern?",
                            ),
                            _av(
                                "hostname", "Returns the name of the current host.", "info hostname"
                            ),
                            _av(
                                "level",
                                "If number is not given, the level this routine was called from.",
                                "info level ?level?",
                            ),
                            _av(
                                "library",
                                "Returns the value of tcl_library, which is the name of the library directory in which the scripts distributed with Tcl scripts are stored.",
                                "info library",
                            ),
                            _av(
                                "loaded",
                                "Returns the name of each file loaded in interp by the load command with prefix prefix .",
                                "info loaded ?interp? ?prefix?",
                            ),
                            _av(
                                "locals",
                                "If pattern is given, returns the name of each local variable matching pattern according to string match.",
                                "info locals ?pattern?",
                            ),
                            _av(
                                "nameofexecutable",
                                "Returns the absolute pathname of the program for the current interpreter.",
                                "info nameofexecutable",
                            ),
                            _av(
                                "patchlevel",
                                "Returns the value of the global variable tcl_patchLevel, in which the exact version of the Tcl library initially stored.",
                                "info patchlevel",
                            ),
                            _av(
                                "procs",
                                "Returns the names of all visible procedures.",
                                "info procs ?pattern?",
                            ),
                            _av(
                                "script",
                                "Returns the pathname of the innermost script currently being evaluated, or the empty string if no pathname can be determined.",
                                "info script ?filename?",
                            ),
                            _av(
                                "sharedlibextension",
                                "Returns the extension used on this platform for names of shared libraries, e.g.",
                                "info sharedlibextension",
                            ),
                            _av(
                                "tclversion",
                                "Returns the value of the global variable tcl_version, in which the major and minor version of the Tcl library are stored.",
                                "info tclversion",
                            ),
                            _av(
                                "vars",
                                "If pattern is not given, returns the names of all visible variables.",
                                "info vars ?pattern?",
                            ),
                            _av(
                                "class",
                                "Returns information about the class named class.",
                                "info class subcommand class ?arg ...",
                            ),
                            _av(
                                "cmdtype",
                                "Returns a the type of the command named commandName.",
                                "info cmdtype commandName",
                            ),
                            _av(
                                "constant",
                                "Returns 1 if varName is a constant variable (see const) and 0 otherwise.",
                                "info constant varName",
                            ),
                            _av(
                                "consts",
                                "Returns the list of constant variables (see const) in the current scope, or the list of constant variables matching pattern (if that is provided) in a manner similar to info vars.",
                                "info consts ?pattern?",
                            ),
                            _av(
                                "coroutine",
                                "Returns the name of the current coroutine, or the empty string if there is no current coroutine or the current coroutine has been deleted.",
                                "info coroutine",
                            ),
                            _av(
                                "errorstack",
                                "Returns a description of the active command at each level for the last error in the current interpreter, or in the interpreter named interp if given.",
                                "info errorstack ?interp?",
                            ),
                            _av(
                                "object",
                                "Returns information about the object named object.",
                                "info object subcommand object ?arg ...",
                            ),
                            _av(
                                "call",
                                "Returns a description of the method implementations that are used to provide a stereotypical instance of class's implementation of method (stereotypical instances being objects instantiated by a class without having any…",
                                "info class call class method",
                            ),
                            _av(
                                "constructor",
                                "This subcommand returns a description of the definition of the constructor of class class.",
                                "info class constructor class",
                            ),
                            _av(
                                "definition",
                                "This subcommand returns a description of the definition of the method named method of class class.",
                                "info class definition class method",
                            ),
                            _av(
                                "definitionnamespace",
                                "This subcommand returns the definition namespace for kind definitions of the class class; the definition namespace only affects the instances of class, not class itself.",
                                "info class definitionnamespace class ?kind?",
                            ),
                            _av(
                                "destructor",
                                "This subcommand returns the body of the destructor of class class.",
                                "info class destructor class",
                            ),
                            _av(
                                "filters",
                                "This subcommand returns the list of filter methods set on the class.",
                                "info class filters class",
                            ),
                            _av(
                                "forward",
                                "This subcommand returns the argument list for the method forwarding called method that is set on the class called class.",
                                "info class forward class method",
                            ),
                            _av(
                                "instances",
                                "This subcommand returns a list of instances of class class.",
                                "info class instances class ?pattern?",
                            ),
                            _av(
                                "methods",
                                "This subcommand returns a list of all public (i.e.",
                                "info class methods class ?options...?",
                            ),
                            _av(
                                "methodtype",
                                "This subcommand returns a description of the type of implementation used for the method named method of class class.",
                                "info class methodtype class method",
                            ),
                            _av(
                                "mixins",
                                "This subcommand returns a list of all classes that have been mixed into the class named class.",
                                "info class mixins class",
                            ),
                            _av(
                                "properties",
                                "This subcommand returns a sorted list of properties defined on the class named class.",
                                "info class properties class ?options...",
                            ),
                            _av(
                                "subclasses",
                                "This subcommand returns a list of direct subclasses of class class.",
                                "info class subclasses class ?pattern?",
                            ),
                            _av(
                                "superclasses",
                                "This subcommand returns a list of direct superclasses of class class in inheritance precedence order.",
                                "info class superclasses class",
                            ),
                            _av(
                                "variables",
                                "This subcommand returns a list of all variables that have been declared for the class named class (i.e.",
                                "info class variables class ?-private?",
                            ),
                            _av(
                                "creationid",
                                "Returns the unique creation identifier for the object object.",
                                "info object creationid object",
                            ),
                            _av(
                                "isa",
                                "This subcommand tests whether an object belongs to a particular category, returning a boolean value that indicates whether the object argument meets the criteria for the category.",
                                "info object isa category object ?arg?",
                            ),
                            _av(
                                "namespace",
                                "This subcommand returns the name of the internal namespace of the object named object.",
                                "info object namespace object",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "args": SubCommand(name="args", arity=Arity(1), return_type=TclType.LIST),
                "body": SubCommand(name="body", arity=Arity(1), return_type=TclType.STRING),
                "cmdcount": SubCommand(name="cmdcount", arity=Arity(0), return_type=TclType.INT),
                "commands": SubCommand(
                    name="commands", arity=Arity(0, 1), return_type=TclType.LIST
                ),
                "complete": SubCommand(
                    name="complete", arity=Arity(1), return_type=TclType.BOOLEAN
                ),
                "default": SubCommand(
                    name="default",
                    arity=Arity(3),
                    arg_roles={2: ArgRole.VAR_NAME},
                    return_type=TclType.BOOLEAN,
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1),
                    arg_roles={0: ArgRole.VAR_READ},
                    return_type=TclType.BOOLEAN,
                ),
                "frame": SubCommand(name="frame", arity=Arity(0, 1), return_type=TclType.DICT),
                "functions": SubCommand(
                    name="functions", arity=Arity(0, 1), return_type=TclType.LIST
                ),
                "globals": SubCommand(name="globals", arity=Arity(0, 1), return_type=TclType.LIST),
                "hostname": SubCommand(name="hostname", arity=Arity(0), return_type=TclType.STRING),
                "level": SubCommand(name="level", arity=Arity(0, 1), return_type=TclType.INT),
                "library": SubCommand(name="library", arity=Arity(0), return_type=TclType.STRING),
                "loaded": SubCommand(name="loaded", arity=Arity(0, 1), return_type=TclType.LIST),
                "locals": SubCommand(name="locals", arity=Arity(0, 1), return_type=TclType.LIST),
                "nameofexecutable": SubCommand(
                    name="nameofexecutable", arity=Arity(0), return_type=TclType.STRING
                ),
                "patchlevel": SubCommand(
                    name="patchlevel", arity=Arity(0), return_type=TclType.STRING
                ),
                "procs": SubCommand(name="procs", arity=Arity(0, 1), return_type=TclType.LIST),
                "script": SubCommand(name="script", arity=Arity(0, 1), return_type=TclType.STRING),
                "sharedlibextension": SubCommand(
                    name="sharedlibextension", arity=Arity(0), return_type=TclType.STRING
                ),
                "tclversion": SubCommand(
                    name="tclversion", arity=Arity(0), return_type=TclType.STRING
                ),
                "vars": SubCommand(name="vars", arity=Arity(0, 1), return_type=TclType.LIST),
            },
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
