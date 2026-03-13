"""font -- Create and inspect fonts."""

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

_SOURCE = "Tk man page font.n"
_av = make_av(_SOURCE)


@register
class FontCommand(CommandDef):
    name = "font"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="font",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and inspect fonts.",
                synopsis=(
                    "font actual font ?-displayof window? ?option?",
                    "font configure fontname ?option? ?value option value ...?",
                    "font create ?fontname? ?option value ...?",
                    "font delete fontname ?fontname ...?",
                    "font families ?-displayof window?",
                    "font measure font ?-displayof window? text",
                    "font metrics font ?-displayof window? ?option?",
                    "font names",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="font option ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-family",
                            takes_value=True,
                            value_hint="name",
                            detail="Font family name (e.g. Courier, Times, Helvetica).",
                        ),
                        OptionSpec(
                            name="-size",
                            takes_value=True,
                            value_hint="size",
                            detail="Desired size of the font in points (positive) or pixels (negative).",
                        ),
                        OptionSpec(
                            name="-weight",
                            takes_value=True,
                            value_hint="normal|bold",
                            detail="Weight of the font: normal or bold.",
                        ),
                        OptionSpec(
                            name="-slant",
                            takes_value=True,
                            value_hint="roman|italic",
                            detail="Slant of the font: roman or italic.",
                        ),
                        OptionSpec(
                            name="-underline",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Whether to draw an underline beneath the text.",
                        ),
                        OptionSpec(
                            name="-overstrike",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Whether to draw a horizontal line through the text.",
                        ),
                        OptionSpec(
                            name="-displayof",
                            takes_value=True,
                            value_hint="window",
                            detail="Specifies the display for the font query.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "actual",
                                "Return the actual attributes of a font on the display.",
                                "font actual font ?-displayof window? ?option?",
                            ),
                            _av(
                                "configure",
                                "Query or modify the desired attributes of a named font.",
                                "font configure fontname ?option? ?value option value ...?",
                            ),
                            _av(
                                "create",
                                "Create a new named font with the given options.",
                                "font create ?fontname? ?option value ...?",
                            ),
                            _av(
                                "delete",
                                "Delete one or more named fonts.",
                                "font delete fontname ?fontname ...?",
                            ),
                            _av(
                                "families",
                                "Return a list of all font families available on the display.",
                                "font families ?-displayof window?",
                            ),
                            _av(
                                "measure",
                                "Measure the width of the text string when rendered in the given font.",
                                "font measure font ?-displayof window? text",
                            ),
                            _av(
                                "metrics",
                                "Return metric information for the given font.",
                                "font metrics font ?-displayof window? ?option?",
                            ),
                            _av(
                                "names",
                                "Return a list of all named fonts currently defined.",
                                "font names",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "actual": SubCommand(
                    name="actual",
                    arity=Arity(1),
                    detail="Return the actual attributes of a font on the display.",
                    synopsis="font actual font ?-displayof window? ?option?",
                ),
                "configure": SubCommand(
                    name="configure",
                    arity=Arity(1),
                    detail="Query or modify the desired attributes of a named font.",
                    synopsis="font configure fontname ?option? ?value option value ...?",
                ),
                "create": SubCommand(
                    name="create",
                    arity=Arity(0),
                    detail="Create a new named font with the given options.",
                    synopsis="font create ?fontname? ?option value ...?",
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(1),
                    detail="Delete one or more named fonts.",
                    synopsis="font delete fontname ?fontname ...?",
                ),
                "families": SubCommand(
                    name="families",
                    arity=Arity(0, 2),
                    detail="Return a list of all font families available on the display.",
                    synopsis="font families ?-displayof window?",
                ),
                "measure": SubCommand(
                    name="measure",
                    arity=Arity(2),
                    detail="Measure the width of the text string when rendered in the given font.",
                    synopsis="font measure font ?-displayof window? text",
                ),
                "metrics": SubCommand(
                    name="metrics",
                    arity=Arity(1),
                    detail="Return metric information for the given font.",
                    synopsis="font metrics font ?-displayof window? ?option?",
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 0),
                    detail="Return a list of all named fonts currently defined.",
                    synopsis="font names",
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
