# Scaffolded from interp.n -- refine and commit
"""interp -- Create and manipulate Tcl interpreters."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page interp.n"


_av = make_av(_SOURCE)


@register
class InterpCommand(CommandDef):
    name = "interp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="interp",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Create and manipulate Tcl interpreters",
                synopsis=("interp subcommand ?arg arg ...?",),
                snippet="This command makes it possible to create one or more new Tcl interpreters that co-exist with the creating interpreter in the same application.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="interp subcommand ?arg arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "alias",
                                "Returns a Tcl list whose elements are the targetCmd and args associated with the alias represented by srcToken (this is the value returned when the alias was created; it is possible that the name of the source command i…",
                                "interp alias srcPath srcToken",
                            ),
                            _av(
                                "aliases",
                                "This command returns a Tcl list of the tokens of all the source commands for aliases defined in the interpreter identified by path.",
                                "interp aliases ?path?",
                            ),
                            _av(
                                "create",
                                "Creates a child interpreter identified by path and a new command, called a child command.",
                                "interp create ?-safe? ?-|-? ?path?",
                            ),
                            _av(
                                "delete",
                                "Deletes zero or more interpreters given by the optional path arguments, and for each interpreter, it also deletes its children.",
                                "interp delete ?path ...?",
                            ),
                            _av(
                                "eval",
                                "This command concatenates all of the arg arguments in the same fashion as the concat command, then evaluates the resulting string as a Tcl script in the child interpreter identified by path.",
                                "interp eval path arg ?arg ...?",
                            ),
                            _av(
                                "exists",
                                "Returns 1 if a child interpreter by the specified path exists in this parent, 0 otherwise.",
                                "interp exists path",
                            ),
                            _av(
                                "expose",
                                "Makes the hidden command hiddenName exposed, eventually bringing it back under a new exposedCmdName name (this name is currently accepted only if it is a valid global name space name without any ::), in the interpreter…",
                                "interp expose path hiddenName ?exposedCmdName?",
                            ),
                            _av(
                                "hidden",
                                "Returns a list of the names of all hidden commands in the interpreter identified by path.",
                                "interp hidden path",
                            ),
                            _av(
                                "hide",
                                "Makes the exposed command exposedCmdName hidden, renaming it to the hidden command hiddenCmdName, or keeping the same name if hiddenCmdName is not given, in the interpreter denoted by path.",
                                "interp hide path exposedCmdName ?hiddenCmdName?",
                            ),
                            _av(
                                "invokehidden",
                                "Invokes the hidden command hiddenCmdName with the arguments supplied in the interpreter denoted by path.",
                                "interp invokehidden path ?-option ...? hiddenCmdName ?arg ...?",
                            ),
                            _av(
                                "issafe",
                                "Returns 1 if the interpreter identified by the specified path is safe, 0 otherwise.",
                                "interp issafe ?path?",
                            ),
                            _av(
                                "limit",
                                "Sets up, manipulates and queries the configuration of the resource limit limitType for the interpreter denoted by path.",
                                "interp limit path limitType ?-option? ?value ...?",
                            ),
                            _av(
                                "marktrusted",
                                "Marks the interpreter identified by path as trusted.",
                                "interp marktrusted path",
                            ),
                            _av(
                                "recursionlimit",
                                "Returns the maximum allowable nesting depth for the interpreter specified by path.",
                                "interp recursionlimit path ?newlimit?",
                            ),
                            _av(
                                "share",
                                "Causes the IO channel identified by channel to become shared between the interpreter identified by srcPath and the interpreter identified by destPath.",
                                "interp share srcPath channel destPath",
                            ),
                            _av("slaves", "interp slaves", "interp slaves"),
                            _av(
                                "target",
                                "Returns a Tcl list describing the target interpreter for an alias.",
                                "interp target path alias",
                            ),
                            _av(
                                "transfer",
                                "Causes the IO channel identified by channel to become available in the interpreter identified by destPath and unavailable in the interpreter identified by srcPath.",
                                "interp transfer srcPath channel destPath",
                            ),
                            _av(
                                "bgerror",
                                "This command either gets or sets the current background exception handler for the interpreter identified by path.",
                                "interp bgerror path ?cmdPrefix?",
                            ),
                            _av(
                                "cancel",
                                "Cancels the script being evaluated in the interpreter identified by path.",
                                "interp cancel ?-unwind? ?-|-? ?path? ?result?",
                            ),
                            _av(
                                "children",
                                "Returns a Tcl list of the names of all the child interpreters associated with the interpreter identified by path.",
                                "interp children ?path?",
                            ),
                            _av(
                                "debug",
                                "Controls whether frame-level stack information is captured in the child interpreter identified by path.",
                                "interp debug path ?-frame ?bool??",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "alias": SubCommand(
                    name="alias",
                    arity=Arity(1),
                    detail="Returns a Tcl list whose elements are the targetCmd and args associated with the alias represented by srcToken (this is the value returned when the alias was created; it is possible that the name of the source command i…",
                    synopsis="interp alias srcPath srcToken",
                    return_type=TclType.STRING,
                ),
                "aliases": SubCommand(
                    name="aliases",
                    arity=Arity(0, 1),
                    detail="This command returns a Tcl list of the tokens of all the source commands for aliases defined in the interpreter identified by path.",
                    synopsis="interp aliases ?path?",
                    return_type=TclType.LIST,
                ),
                "create": SubCommand(
                    name="create",
                    arity=Arity(0),
                    detail="Creates a child interpreter identified by path and a new command, called a child command.",
                    synopsis="interp create ?-safe? ?-|-? ?path?",
                    return_type=TclType.STRING,
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(1),
                    detail="Deletes zero or more interpreters given by the optional path arguments, and for each interpreter, it also deletes its children.",
                    synopsis="interp delete ?path ...?",
                    return_type=TclType.STRING,
                ),
                "eval": SubCommand(
                    name="eval",
                    arity=Arity(2),
                    detail="This command concatenates all of the arg arguments in the same fashion as the concat command, then evaluates the resulting string as a Tcl script in the child interpreter identified by path.",
                    synopsis="interp eval path arg ?arg ...?",
                    return_type=TclType.STRING,
                    arg_roles={1: ArgRole.BODY},
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Returns 1 if a child interpreter by the specified path exists in this parent, 0 otherwise.",
                    synopsis="interp exists path",
                    return_type=TclType.BOOLEAN,
                ),
                "expose": SubCommand(
                    name="expose",
                    arity=Arity(2, 3),
                    detail="Makes the hidden command hiddenName exposed, eventually bringing it back under a new exposedCmdName name (this name is currently accepted only if it is a valid global name space name without any ::), in the interpreter…",
                    synopsis="interp expose path hiddenName ?exposedCmdName?",
                    return_type=TclType.STRING,
                ),
                "hidden": SubCommand(
                    name="hidden",
                    arity=Arity(0, 1),
                    detail="Returns a list of the names of all hidden commands in the interpreter identified by path.",
                    synopsis="interp hidden path",
                    return_type=TclType.LIST,
                ),
                "hide": SubCommand(
                    name="hide",
                    arity=Arity(2, 3),
                    detail="Makes the exposed command exposedCmdName hidden, renaming it to the hidden command hiddenCmdName, or keeping the same name if hiddenCmdName is not given, in the interpreter denoted by path.",
                    synopsis="interp hide path exposedCmdName ?hiddenCmdName?",
                    return_type=TclType.STRING,
                ),
                "invokehidden": SubCommand(
                    name="invokehidden",
                    arity=Arity(2),
                    detail="Invokes the hidden command hiddenCmdName with the arguments supplied in the interpreter denoted by path.",
                    synopsis="interp invokehidden path ?-option ...? hiddenCmdName ?arg ...?",
                    return_type=TclType.STRING,
                ),
                "issafe": SubCommand(
                    name="issafe",
                    arity=Arity(0, 1),
                    detail="Returns 1 if the interpreter identified by the specified path is safe, 0 otherwise.",
                    synopsis="interp issafe ?path?",
                    return_type=TclType.BOOLEAN,
                ),
                "limit": SubCommand(
                    name="limit",
                    arity=Arity(2),
                    detail="Sets up, manipulates and queries the configuration of the resource limit limitType for the interpreter denoted by path.",
                    synopsis="interp limit path limitType ?-option? ?value ...?",
                    return_type=TclType.STRING,
                ),
                "marktrusted": SubCommand(
                    name="marktrusted",
                    arity=Arity(1, 1),
                    detail="Marks the interpreter identified by path as trusted.",
                    synopsis="interp marktrusted path",
                    return_type=TclType.STRING,
                ),
                "recursionlimit": SubCommand(
                    name="recursionlimit",
                    arity=Arity(1, 2),
                    detail="Returns the maximum allowable nesting depth for the interpreter specified by path.",
                    synopsis="interp recursionlimit path ?newlimit?",
                    return_type=TclType.INT,
                ),
                "share": SubCommand(
                    name="share",
                    arity=Arity(3, 3),
                    detail="Causes the IO channel identified by channel to become shared between the interpreter identified by srcPath and the interpreter identified by destPath.",
                    synopsis="interp share srcPath channel destPath",
                    return_type=TclType.STRING,
                ),
                "slaves": SubCommand(
                    name="slaves",
                    arity=Arity(0, 1),
                    detail="interp slaves",
                    synopsis="interp slaves",
                    return_type=TclType.LIST,
                ),
                "target": SubCommand(
                    name="target",
                    arity=Arity(2, 2),
                    detail="Returns a Tcl list describing the target interpreter for an alias.",
                    synopsis="interp target path alias",
                    return_type=TclType.STRING,
                ),
                "transfer": SubCommand(
                    name="transfer",
                    arity=Arity(3, 3),
                    detail="Causes the IO channel identified by channel to become available in the interpreter identified by destPath and unavailable in the interpreter identified by srcPath.",
                    synopsis="interp transfer srcPath channel destPath",
                    return_type=TclType.STRING,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_interp_eval_subcommands=frozenset({"eval", "invokehidden"}),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
