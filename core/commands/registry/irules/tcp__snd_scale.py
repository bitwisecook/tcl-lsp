# Enriched from F5 iRules reference documentation.
"""TCP::snd_scale -- Returns the receive window scale advertised by the local host."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__snd_scale.html"


@register
class TcpSndScaleCommand(CommandDef):
    name = "TCP::snd_scale"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::snd_scale",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the receive window scale advertised by the local host.",
                synopsis=("TCP::snd_scale",),
                snippet="Returns the receive window scale advertised by the local host.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Log snd_scale.\n"
                    '    log local0. "snd_scale: [TCP::snd_scale]"\n'
                    "}"
                ),
                return_value="The bitshift associated with the local host window scale.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::snd_scale",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
