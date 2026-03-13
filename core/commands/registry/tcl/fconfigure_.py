"""fconfigure -- Set and get options on a channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl fconfigure(1)"


@register
class FconfigureCommand(CommandDef):
    name = "fconfigure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fconfigure",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Set and get options on a channel.",
                synopsis=("fconfigure channelId ?optionName? ?value ...?",),
                snippet="",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fconfigure channelId ?optionName? ?value ...?",
                    options=(
                        OptionSpec(name="-blocking", takes_value=True, value_hint="boolean"),
                        OptionSpec(name="-buffering", takes_value=True, value_hint="mode"),
                        OptionSpec(name="-buffersize", takes_value=True, value_hint="size"),
                        OptionSpec(name="-encoding", takes_value=True, value_hint="encoding"),
                        OptionSpec(name="-eofchar", takes_value=True, value_hint="chars"),
                        OptionSpec(name="-translation", takes_value=True, value_hint="mode"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
