# Scaffolded from array.n -- refine and commit
"""array -- Manipulate array variables."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page array.n"


_av = make_av(_SOURCE)


@register
class ArrayCommand(CommandDef):
    name = "array"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="array",
            hover=HoverSnippet(
                summary="Manipulate array variables",
                synopsis=("array option arrayName ?arg arg ...?",),
                snippet="This command performs one of several operations on the variable given by arrayName.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="array option arrayName ?arg arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "anymore",
                                "Returns 1 if there are any more elements left to be processed in an array search, 0 if all elements have already been returned.",
                                "array anymore arrayName searchId",
                            ),
                            _av(
                                "donesearch",
                                "This command terminates an array search and destroys all the state associated with that search.",
                                "array donesearch arrayName searchId",
                            ),
                            _av(
                                "exists",
                                "Returns 1 if arrayName is an array variable, 0 if there is no variable by that name or if it is a scalar variable.",
                                "array exists arrayName",
                            ),
                            _av(
                                "get",
                                "Returns a list containing pairs of elements.",
                                "array get arrayName ?pattern?",
                            ),
                            _av(
                                "names",
                                "Returns a list containing the names of all of the elements in the array that match pattern.",
                                "array names arrayName ?mode? ?pattern?",
                            ),
                            _av(
                                "nextelement",
                                "Returns the name of the next element in arrayName, or an empty string if all elements of arrayName have already been returned in this search.",
                                "array nextelement arrayName searchId",
                            ),
                            _av(
                                "set",
                                "Sets the values of one or more elements in arrayName.",
                                "array set arrayName list",
                            ),
                            _av(
                                "size",
                                "Returns a decimal string giving the number of elements in the array.",
                                "array size arrayName",
                            ),
                            _av(
                                "startsearch",
                                "This command initializes an element-by-element search through the array given by arrayName, such that invocations of the array nextelement command will return the names of the individual elements in the array.",
                                "array startsearch arrayName",
                            ),
                            _av(
                                "statistics",
                                "Returns statistics about the distribution of data within the hashtable that represents the array.",
                                "array statistics arrayName",
                            ),
                            _av(
                                "unset",
                                "Unsets all of the elements in the array that match pattern (using the matching rules of string match).",
                                "array unset arrayName ?pattern?",
                            ),
                            _av(
                                "default",
                                "Manages the default value of the array.",
                                "array default subcommand arrayName args...",
                            ),
                            _av(
                                "for",
                                "The first argument is a two element list of variable names for the key and value of each entry in the array.",
                                "array for {keyVariable valueVariable} arrayName body",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "anymore": SubCommand(
                    name="anymore",
                    arity=Arity(2, 2),
                    detail="Returns 1 if there are any more elements left to be processed in an array search, 0 if all elements have already been returned.",
                    synopsis="array anymore arrayName searchId",
                    return_type=TclType.BOOLEAN,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "donesearch": SubCommand(
                    name="donesearch",
                    arity=Arity(2, 2),
                    detail="This command terminates an array search and destroys all the state associated with that search.",
                    synopsis="array donesearch arrayName searchId",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Returns 1 if arrayName is an array variable, 0 if there is no variable by that name or if it is a scalar variable.",
                    synopsis="array exists arrayName",
                    return_type=TclType.BOOLEAN,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "get": SubCommand(
                    name="get",
                    arity=Arity(1, 2),
                    detail="Returns a list containing pairs of elements.",
                    synopsis="array get arrayName ?pattern?",
                    return_type=TclType.LIST,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(1, 3),
                    detail="Returns a list containing the names of all of the elements in the array that match pattern.",
                    synopsis="array names arrayName ?mode? ?pattern?",
                    return_type=TclType.LIST,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "nextelement": SubCommand(
                    name="nextelement",
                    arity=Arity(2, 2),
                    detail="Returns the name of the next element in arrayName, or an empty string if all elements of arrayName have already been returned in this search.",
                    synopsis="array nextelement arrayName searchId",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "set": SubCommand(
                    name="set",
                    arity=Arity(2, 2),
                    detail="Sets the values of one or more elements in arrayName.",
                    synopsis="array set arrayName list",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
                "size": SubCommand(
                    name="size",
                    arity=Arity(1, 1),
                    detail="Returns a decimal string giving the number of elements in the array.",
                    synopsis="array size arrayName",
                    return_type=TclType.INT,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "startsearch": SubCommand(
                    name="startsearch",
                    arity=Arity(1, 1),
                    detail="This command initializes an element-by-element search through the array given by arrayName, such that invocations of the array nextelement command will return the names of the individual elements in the array.",
                    synopsis="array startsearch arrayName",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "statistics": SubCommand(
                    name="statistics",
                    arity=Arity(1, 1),
                    detail="Returns statistics about the distribution of data within the hashtable that represents the array.",
                    synopsis="array statistics arrayName",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.VAR_READ},
                ),
                "unset": SubCommand(
                    name="unset",
                    arity=Arity(1, 2),
                    detail="Unsets all of the elements in the array that match pattern (using the matching rules of string match).",
                    synopsis="array unset arrayName ?pattern?",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
