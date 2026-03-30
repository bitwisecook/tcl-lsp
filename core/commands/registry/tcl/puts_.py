"""puts -- Write text to a channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl puts(1)"


@register
class PutsCommand(CommandDef):
    name = "puts"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="puts",
            hover=HoverSnippet(
                summary="Write text to a channel (stdout by default).",
                synopsis=("puts ?-nonewline? ?channelId? string",),
                snippet="Use `-nonewline` to suppress the trailing newline.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="puts ?-nonewline? ?channelId? string",
                    options=(
                        OptionSpec(
                            name="-nonewline",
                            detail="Do not append trailing newline.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            taint_output_sink="T101",
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
