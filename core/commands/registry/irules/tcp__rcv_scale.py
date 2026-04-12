# Enriched from F5 iRules reference documentation.
"""TCP::rcv_scale -- Returns the receive window scale advertised by the remote host."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rcv_scale.html"


@register
class TcpRcvScaleCommand(CommandDef):
    name = "TCP::rcv_scale"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rcv_scale",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the receive window scale advertised by the remote host.",
                synopsis=("TCP::rcv_scale",),
                snippet="Returns the receive window scale advertised by the remote host.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Log rcv_scale.\n"
                    '    log local0. "rcv_scale: [TCP::rcv_scale]"\n'
                    "}"
                ),
                return_value="The bitshift associated with the remote host window scale.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rcv_scale",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            excluded_events=("SERVER_INIT",),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
