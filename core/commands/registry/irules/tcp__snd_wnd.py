# Enriched from F5 iRules reference documentation.
"""TCP::snd_wnd -- The remote host's advertised receive window."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__snd_wnd.html"


@register
class TcpSndWndCommand(CommandDef):
    name = "TCP::snd_wnd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::snd_wnd",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="The remote host's advertised receive window.",
                synopsis=("TCP::snd_wnd",),
                snippet=(
                    "Returns the remote host's advertised receive window. If smaller\n"
                    "than the congestion window (cwnd) and send buffer size, this limits\n"
                    "the amount of outstanding data on the connection."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_CLOSED {\n"
                    "    # Get Client's last advertised window.\n"
                    '    log local0. "Client\'s advertised rwnd: [TCP::snd_wnd]"\n'
                    "}"
                ),
                return_value="The advertised receive window (rwnd) in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::snd_wnd",
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
