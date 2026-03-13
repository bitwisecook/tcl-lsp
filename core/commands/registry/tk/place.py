"""place -- Geometry manager for fixed or rubber-sheet placement."""

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

_SOURCE = "Tk man page place.n"
_av = make_av(_SOURCE)


@register
class PlaceCommand(CommandDef):
    name = "place"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="place",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Geometry manager for fixed or rubber-sheet placement.",
                synopsis=(
                    "place window option value ?option value ...?",
                    "place configure window ?option? ?value option value ...?",
                    "place forget window",
                    "place info window",
                    "place slaves window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="place option arg ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-x",
                            takes_value=True,
                            value_hint="location",
                            detail="Specifies the x-coordinate of the anchor point in the master window.",
                        ),
                        OptionSpec(
                            name="-y",
                            takes_value=True,
                            value_hint="location",
                            detail="Specifies the y-coordinate of the anchor point in the master window.",
                        ),
                        OptionSpec(
                            name="-relx",
                            takes_value=True,
                            value_hint="location",
                            detail="Specifies the x-coordinate as a fraction of the master width (0.0 to 1.0).",
                        ),
                        OptionSpec(
                            name="-rely",
                            takes_value=True,
                            value_hint="location",
                            detail="Specifies the y-coordinate as a fraction of the master height (0.0 to 1.0).",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="size",
                            detail="Specifies the width of the slave in screen units.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            value_hint="size",
                            detail="Specifies the height of the slave in screen units.",
                        ),
                        OptionSpec(
                            name="-relwidth",
                            takes_value=True,
                            value_hint="size",
                            detail="Specifies the width as a fraction of the master width (0.0 to 1.0).",
                        ),
                        OptionSpec(
                            name="-relheight",
                            takes_value=True,
                            value_hint="size",
                            detail="Specifies the height as a fraction of the master height (0.0 to 1.0).",
                        ),
                        OptionSpec(
                            name="-anchor",
                            takes_value=True,
                            value_hint="n|ne|e|se|s|sw|w|nw|center",
                            detail="Specifies which point of the slave is positioned at the (x,y) location.",
                        ),
                        OptionSpec(
                            name="-bordermode",
                            takes_value=True,
                            value_hint="inside|outside|ignore",
                            detail="Determines the degree to which borders within the master are used.",
                        ),
                        OptionSpec(
                            name="-in",
                            takes_value=True,
                            value_hint="master",
                            detail="Specifies the master window relative to which the slave is placed.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "configure",
                                "Set or query the placement options for a window.",
                                "place configure window ?option? ?value option value ...?",
                            ),
                            _av(
                                "forget",
                                "Cause the placer to stop managing the geometry of the window.",
                                "place forget window",
                            ),
                            _av(
                                "info",
                                "Return a list of the current configuration for the window.",
                                "place info window",
                            ),
                            _av(
                                "slaves",
                                "Return a list of all slaves managed by the placer for the window.",
                                "place slaves window",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "configure": SubCommand(
                    name="configure",
                    arity=Arity(1),
                    detail="Set or query the placement options for a window.",
                    synopsis="place configure window ?option? ?value option value ...?",
                ),
                "forget": SubCommand(
                    name="forget",
                    arity=Arity(1, 1),
                    detail="Cause the placer to stop managing the geometry of the window.",
                    synopsis="place forget window",
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(1, 1),
                    detail="Return a list of the current configuration for the window.",
                    synopsis="place info window",
                ),
                "slaves": SubCommand(
                    name="slaves",
                    arity=Arity(1, 1),
                    detail="Return a list of all slaves managed by the placer for the window.",
                    synopsis="place slaves window",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
