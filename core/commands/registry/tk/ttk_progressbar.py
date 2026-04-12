"""ttk::progressbar -- Themed progress indicator widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_progressbar.n"

_av = make_av(_SOURCE)

_ORIENT_VALUES = (
    _av("horizontal", "Progress bar fills from left to right."),
    _av("vertical", "Progress bar fills from bottom to top."),
)

_MODE_VALUES = (
    _av("determinate", "Progress bar shows a fixed fraction of completion."),
    _av("indeterminate", "Progress bar animates to indicate activity."),
)


@register
class TtkProgressbarCommand(CommandDef):
    name = "ttk::progressbar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::progressbar",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed progress indicator widget.",
                synopsis=("ttk::progressbar pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::progressbar pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-orient",
                            takes_value=True,
                            value_hint="orientation",
                            detail="Orientation of the progress bar (horizontal or vertical).",
                        ),
                        OptionSpec(
                            name="-length",
                            takes_value=True,
                            value_hint="length",
                            detail="Length of the long axis of the progress bar.",
                        ),
                        OptionSpec(
                            name="-mode",
                            takes_value=True,
                            value_hint="progressMode",
                            detail="Mode of the progress bar (determinate or indeterminate).",
                        ),
                        OptionSpec(
                            name="-maximum",
                            takes_value=True,
                            value_hint="maximum",
                            detail="Maximum value of the progress bar.",
                        ),
                        OptionSpec(
                            name="-value",
                            takes_value=True,
                            value_hint="value",
                            detail="Current value of the progress bar.",
                        ),
                        OptionSpec(
                            name="-variable",
                            takes_value=True,
                            value_hint="varName",
                            detail="Variable linked to the progress bar value.",
                        ),
                        OptionSpec(
                            name="-phase",
                            takes_value=True,
                            value_hint="phase",
                            detail="Read-only value used by the theme engine for animation.",
                        ),
                        OptionSpec(
                            name="-style",
                            takes_value=True,
                            value_hint="style",
                            detail="Style to use for the widget.",
                        ),
                        OptionSpec(
                            name="-class",
                            takes_value=True,
                            value_hint="className",
                            detail="Widget class name for option-database lookups.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            value_hint="cursor",
                            detail="Cursor to display when the pointer is over the widget.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            value_hint="focusSpec",
                            detail="Whether the widget accepts focus during keyboard traversal.",
                        ),
                    ),
                ),
            ),
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
