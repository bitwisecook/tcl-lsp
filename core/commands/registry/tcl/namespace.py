# Scaffolded from namespace.n -- refine and commit
"""namespace -- create and manipulate contexts for commands and variables."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page namespace.n"


_av = make_av(_SOURCE)


@register
class NamespaceCommand(CommandDef):
    name = "namespace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="namespace",
            dialects=DIALECTS_EXCEPT_IRULES,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="create and manipulate contexts for commands and variables",
                synopsis=("namespace subcommand ?arg ...?",),
                snippet="The namespace command lets you create, access, and destroy separate contexts for commands and variables.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="namespace subcommand ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "children",
                                "Returns a list of all child namespaces that belong to the namespace namespace.",
                                "namespace children ?namespace? ?pattern?",
                            ),
                            _av(
                                "code",
                                "Captures the current namespace context for later execution of the script script.",
                                "namespace code script",
                            ),
                            _av(
                                "current",
                                "Returns the fully-qualified name for the current namespace.",
                                "namespace current",
                            ),
                            _av(
                                "delete",
                                "Each namespace namespace is deleted and all variables, procedures, and child namespaces contained in the namespace are deleted.",
                                "namespace delete ?namespace namespace ...?",
                            ),
                            _av(
                                "ensemble",
                                "Creates and manipulates a command that is formed out of an ensemble of subcommands.",
                                "namespace ensemble subcommand ?arg ...?",
                            ),
                            _av(
                                "eval",
                                "Activates a namespace called namespace and evaluates some code in that context.",
                                "namespace eval namespace arg ?arg ...?",
                            ),
                            _av(
                                "exists",
                                "Returns 1 if namespace is a valid namespace in the current context, returns 0 otherwise.",
                                "namespace exists namespace",
                            ),
                            _av(
                                "export",
                                "Specifies which commands are exported from a namespace.",
                                "namespace export ?-clear? ?pattern pattern ...?",
                            ),
                            _av(
                                "import",
                                "Imports commands into a namespace, or queries the set of imported commands in a namespace.",
                                "namespace import ?-force? ?pattern pattern ...?",
                            ),
                            _av(
                                "inscope",
                                "Executes a script in the context of the specified namespace.",
                                "namespace inscope namespace script ?arg ...?",
                            ),
                            _av(
                                "origin",
                                "Returns the fully-qualified name of the original command to which the imported command command refers.",
                                "namespace origin command",
                            ),
                            _av(
                                "parent",
                                "Returns the fully-qualified name of the parent namespace for namespace namespace.",
                                "namespace parent ?namespace?",
                            ),
                            _av(
                                "path",
                                "Returns the command resolution path of the current namespace.",
                                "namespace path ?namespaceList?",
                            ),
                            _av(
                                "qualifiers",
                                "Returns any leading namespace qualifiers for string.",
                                "namespace qualifiers string",
                            ),
                            _av(
                                "tail",
                                "Returns the simple name at the end of a qualified string.",
                                "namespace tail string",
                            ),
                            _av(
                                "unknown",
                                "Sets or returns the unknown command handler for the current namespace.",
                                "namespace unknown ?script?",
                            ),
                            _av(
                                "upvar",
                                "This command arranges for zero or more local variables in the current procedure to refer to variables in namespace.",
                                "namespace upvar namespace ?otherVar myVar ...?",
                            ),
                            _av(
                                "which",
                                "Looks up name as either a command or variable and returns its fully-qualified name.",
                                "namespace which ?-command? ?-variable? name",
                            ),
                            _av(
                                "forget",
                                "Removes previously imported commands from a namespace.",
                                "namespace forget ?pattern pattern ...?",
                            ),
                            _av(
                                "create",
                                "Creates a new ensemble command linked to the current namespace, returning the fully qualified name of the command created.",
                                "namespace ensemble create ?option value ...?",
                            ),
                            _av(
                                "configure",
                                "Retrieves the value of an option associated with the ensemble command named command, or updates some options associated with that ensemble command.",
                                "namespace ensemble configure command ?option? ?value ...?",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "children": SubCommand(
                    name="children",
                    arity=Arity(0, 2),
                    detail="Returns a list of all child namespaces that belong to the namespace namespace.",
                    synopsis="namespace children ?namespace? ?pattern?",
                    return_type=TclType.LIST,
                ),
                "code": SubCommand(
                    name="code",
                    arity=Arity(1, 1),
                    detail="Captures the current namespace context for later execution of the script script.",
                    synopsis="namespace code script",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.BODY},
                ),
                "current": SubCommand(
                    name="current",
                    arity=Arity(0, 0),
                    detail="Returns the fully-qualified name for the current namespace.",
                    synopsis="namespace current",
                    return_type=TclType.STRING,
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(0),
                    detail="Each namespace namespace is deleted and all variables, procedures, and child namespaces contained in the namespace are deleted.",
                    synopsis="namespace delete ?namespace namespace ...?",
                    return_type=TclType.STRING,
                ),
                "ensemble": SubCommand(
                    name="ensemble",
                    arity=Arity(1),
                    detail="Creates and manipulates a command that is formed out of an ensemble of subcommands.",
                    synopsis="namespace ensemble subcommand ?arg ...?",
                    return_type=TclType.STRING,
                ),
                "eval": SubCommand(
                    name="eval",
                    arity=Arity(2),
                    detail="Activates a namespace called namespace and evaluates some code in that context.",
                    synopsis="namespace eval namespace arg ?arg ...?",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.NAME, 1: ArgRole.BODY},
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Returns 1 if namespace is a valid namespace in the current context, returns 0 otherwise.",
                    synopsis="namespace exists namespace",
                    return_type=TclType.BOOLEAN,
                ),
                "export": SubCommand(
                    name="export",
                    arity=Arity(0),
                    detail="Specifies which commands are exported from a namespace.",
                    synopsis="namespace export ?-clear? ?pattern pattern ...?",
                    return_type=TclType.STRING,
                ),
                "import": SubCommand(
                    name="import",
                    arity=Arity(0),
                    detail="Imports commands into a namespace, or queries the set of imported commands in a namespace.",
                    synopsis="namespace import ?-force? ?pattern pattern ...?",
                    return_type=TclType.STRING,
                ),
                "inscope": SubCommand(
                    name="inscope",
                    arity=Arity(2),
                    detail="Executes a script in the context of the specified namespace.",
                    synopsis="namespace inscope namespace script ?arg ...?",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.NAME, 1: ArgRole.BODY},
                ),
                "origin": SubCommand(
                    name="origin",
                    arity=Arity(1, 1),
                    detail="Returns the fully-qualified name of the original command to which the imported command command refers.",
                    synopsis="namespace origin command",
                    return_type=TclType.STRING,
                ),
                "parent": SubCommand(
                    name="parent",
                    arity=Arity(0, 1),
                    detail="Returns the fully-qualified name of the parent namespace for namespace namespace.",
                    synopsis="namespace parent ?namespace?",
                    return_type=TclType.STRING,
                ),
                "path": SubCommand(
                    name="path",
                    arity=Arity(0, 1),
                    detail="Returns the command resolution path of the current namespace.",
                    synopsis="namespace path ?namespaceList?",
                    return_type=TclType.LIST,
                ),
                "qualifiers": SubCommand(
                    name="qualifiers",
                    arity=Arity(1, 1),
                    detail="Returns any leading namespace qualifiers for string.",
                    synopsis="namespace qualifiers string",
                    return_type=TclType.STRING,
                ),
                "tail": SubCommand(
                    name="tail",
                    arity=Arity(1, 1),
                    detail="Returns the simple name at the end of a qualified string.",
                    synopsis="namespace tail string",
                    return_type=TclType.STRING,
                ),
                "unknown": SubCommand(
                    name="unknown",
                    arity=Arity(0, 1),
                    detail="Sets or returns the unknown command handler for the current namespace.",
                    synopsis="namespace unknown ?script?",
                    return_type=TclType.STRING,
                ),
                "upvar": SubCommand(
                    name="upvar",
                    arity=Arity(2),
                    detail="This command arranges for zero or more local variables in the current procedure to refer to variables in namespace.",
                    synopsis="namespace upvar namespace ?otherVar myVar ...?",
                    return_type=TclType.STRING,
                ),
                "which": SubCommand(
                    name="which",
                    arity=Arity(1),
                    detail="Looks up name as either a command or variable and returns its fully-qualified name.",
                    synopsis="namespace which ?-command? ?-variable? name",
                    return_type=TclType.STRING,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NAMESPACE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
