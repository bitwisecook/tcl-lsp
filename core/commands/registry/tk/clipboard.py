"""clipboard -- Manipulate the Tk clipboard."""

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

_SOURCE = "Tk man page clipboard.n"
_av = make_av(_SOURCE)


@register
class ClipboardCommand(CommandDef):
    name = "clipboard"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="clipboard",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Manipulate the Tk clipboard.",
                synopsis=(
                    "clipboard append ?-displayof window? ?-format format? ?-type type? data",
                    "clipboard clear ?-displayof window?",
                    "clipboard get ?-displayof window? ?-type type?",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="clipboard option ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-displayof",
                            takes_value=True,
                            value_hint="window",
                            detail="Specifies the display for the clipboard operation.",
                        ),
                        OptionSpec(
                            name="-format",
                            takes_value=True,
                            value_hint="format",
                            detail="Specifies the representation format for the data (append).",
                        ),
                        OptionSpec(
                            name="-type",
                            takes_value=True,
                            value_hint="type",
                            detail="Specifies the form in which the selection is to be returned.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "append",
                                "Append data to the clipboard on the specified display.",
                                "clipboard append ?-displayof window? ?-format format? ?-type type? data",
                            ),
                            _av(
                                "clear",
                                "Claim ownership of the clipboard and clear its contents.",
                                "clipboard clear ?-displayof window?",
                            ),
                            _av(
                                "get",
                                "Retrieve data from the clipboard on the specified display.",
                                "clipboard get ?-displayof window? ?-type type?",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "append": SubCommand(
                    name="append",
                    arity=Arity(1),
                    detail="Append data to the clipboard on the specified display.",
                    synopsis="clipboard append ?-displayof window? ?-format format? ?-type type? data",
                ),
                "clear": SubCommand(
                    name="clear",
                    arity=Arity(0),
                    detail="Claim ownership of the clipboard and clear its contents.",
                    synopsis="clipboard clear ?-displayof window?",
                ),
                "get": SubCommand(
                    name="get",
                    arity=Arity(0),
                    detail="Retrieve data from the clipboard on the specified display.",
                    synopsis="clipboard get ?-displayof window? ?-type type?",
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
