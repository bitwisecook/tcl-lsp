# Enriched from F5 iRules reference documentation.
"""TCP::rto -- Returns the current value of Retransmission timeout."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rto.html"


@register
class TcpRtoCommand(CommandDef):
    name = "TCP::rto"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rto",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current value of Retransmission timeout.",
                synopsis=("TCP::rto",),
                snippet="Returns the last setting to which the retransmit timer was set in milliseconds. It does not include time elapsed since the timer was set.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    set rto [TCP::rto]\n"
                    '    log local0. "Final RTO value is $rto"\n'
                    "}"
                ),
                return_value="Retransmit timer value in milliseconds.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rto",
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
