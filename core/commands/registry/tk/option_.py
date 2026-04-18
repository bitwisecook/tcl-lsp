"""option -- Add/retrieve window options to/from the option database."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page option.n"
_av = make_av(_SOURCE)


@register
class OptionCommand(CommandDef):
    name = "option"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="option",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Add or retrieve window options to or from the option database.",
                synopsis=(
                    "option add pattern value ?priority?",
                    "option clear",
                    "option get window name class",
                    "option readfile fileName ?priority?",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="option option ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "add",
                                "Add an option to the database with optional priority.",
                                "option add pattern value ?priority?",
                            ),
                            _av(
                                "clear",
                                "Clear all options from the database.",
                                "option clear",
                            ),
                            _av(
                                "get",
                                "Retrieve the value of the option for a window.",
                                "option get window name class",
                            ),
                            _av(
                                "readfile",
                                "Read options from a file and add them to the database.",
                                "option readfile fileName ?priority?",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(2, 3),
                    detail="Add an option to the database with optional priority.",
                    synopsis="option add pattern value ?priority?",
                ),
                "clear": SubCommand(
                    name="clear",
                    arity=Arity(0, 0),
                    detail="Clear all options from the database.",
                    synopsis="option clear",
                ),
                "get": SubCommand(
                    name="get",
                    arity=Arity(3, 3),
                    detail="Retrieve the value of the option for a window.",
                    synopsis="option get window name class",
                ),
                "readfile": SubCommand(
                    name="readfile",
                    arity=Arity(1, 2),
                    detail="Read options from a file and add them to the database.",
                    synopsis="option readfile fileName ?priority?",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
