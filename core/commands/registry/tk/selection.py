"""selection -- Manipulate the X selection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page selection.n"
_av = make_av(_SOURCE)


@register
class SelectionCommand(CommandDef):
    name = "selection"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="selection",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Manipulate the X selection.",
                synopsis=(
                    "selection clear ?-displayof window? ?-selection selection?",
                    "selection get ?-displayof window? ?-selection selection? ?-type type?",
                    "selection handle ?-selection selection? ?-type type? ?-format format? window command",
                    "selection own ?-displayof window? ?-selection selection?",
                    "selection own ?-command command? ?-selection selection? window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="selection option ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-displayof",
                            takes_value=True,
                            value_hint="window",
                            detail="Specifies the display for the selection operation.",
                        ),
                        OptionSpec(
                            name="-selection",
                            takes_value=True,
                            value_hint="selection",
                            detail="Specifies which named selection to operate on (default: PRIMARY).",
                        ),
                        OptionSpec(
                            name="-type",
                            takes_value=True,
                            value_hint="type",
                            detail="Specifies the form in which the selection is to be returned.",
                        ),
                        OptionSpec(
                            name="-format",
                            takes_value=True,
                            value_hint="format",
                            detail="Specifies the representation format for the selection data.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            value_hint="command",
                            detail="Specifies a Tcl script to run when the selection is claimed by another window.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "clear",
                                "Clear the selection so that no window owns it.",
                                "selection clear ?-displayof window? ?-selection selection?",
                            ),
                            _av(
                                "get",
                                "Retrieve the selection and return it as a string.",
                                "selection get ?-displayof window? ?-selection selection? ?-type type?",
                            ),
                            _av(
                                "handle",
                                "Register a handler to provide the selection data.",
                                "selection handle ?-selection sel? ?-type type? ?-format fmt? window command",
                            ),
                            _av(
                                "own",
                                "Query or set the owner of the selection.",
                                "selection own ?-command command? ?-selection selection? window",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "clear": SubCommand(
                    name="clear",
                    arity=Arity(0),
                    detail="Clear the selection so that no window owns it.",
                    synopsis="selection clear ?-displayof window? ?-selection selection?",
                ),
                "get": SubCommand(
                    name="get",
                    arity=Arity(0),
                    detail="Retrieve the selection and return it as a string.",
                    synopsis="selection get ?-displayof window? ?-selection selection? ?-type type?",
                ),
                "handle": SubCommand(
                    name="handle",
                    arity=Arity(2),
                    detail="Register a handler to provide the selection data.",
                    synopsis="selection handle ?-selection sel? ?-type type? ?-format fmt? window command",
                ),
                "own": SubCommand(
                    name="own",
                    arity=Arity(0),
                    detail="Query or set the owner of the selection.",
                    synopsis="selection own ?-command command? ?-selection selection? window",
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
