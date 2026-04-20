# Enriched from F5 iRules reference documentation.
"""TCP::snd_cwnd -- Returns the TCP congestion window (cwnd)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__snd_cwnd.html"


@register
class TcpSndCwndCommand(CommandDef):
    name = "TCP::snd_cwnd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::snd_cwnd",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCP congestion window (cwnd).",
                synopsis=("TCP::snd_cwnd",),
                snippet=(
                    "Returns the TCP congestion window (cwnd), the maximum\n"
                    "unacknowledged data the connection can send due to the congestion\n"
                    "control algorithm.\n"
                    "\n"
                    "The actual amount of outstanding data may be lower, due to lack of\n"
                    "application data to send, the remote host's advertised receive\n"
                    "window, or the size of the BIG-IP send buffer."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # Get BIGIP's last congestion window.\n"
                    '    log local0. "BIGIP\'s cwnd: [TCP::snd_cwnd]"\n'
                    "}"
                ),
                return_value="The cwnd in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::snd_cwnd",
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
