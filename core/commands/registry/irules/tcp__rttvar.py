# Enriched from F5 iRules reference documentation.
"""TCP::rttvar -- Returns TCP's smoothed RTT variance estimate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rttvar.html"


@register
class TcpRttvarCommand(CommandDef):
    name = "TCP::rttvar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rttvar",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns TCP's smoothed RTT variance estimate.",
                synopsis=("TCP::rttvar",),
                snippet=(
                    "Returns the Round Trip Time Variance, which is an indication of path jitter. TCP uses this figure, combined with RTT, to compute the RTO.\n"
                    "\n"
                    'Note that the value returned is in units of "1/16 of a millisecond". Divide the returned value by 16 to get the actual variance in milliseconds.'
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # Log rttvar.\n"
                    '    log local0. "rttvar: [TCP::rttvar]"\n'
                    "}"
                ),
                return_value='The measured RTT variance in units of "1/16 of a millisecond". Divide the returned value by 16 to get the actual variance in milliseconds.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rttvar",
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
