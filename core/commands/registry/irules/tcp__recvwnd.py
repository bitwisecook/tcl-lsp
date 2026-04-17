# Enriched from F5 iRules reference documentation.
"""TCP::recvwnd -- This command can be used to set/get the receive window size of a TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__recvwnd.html"


_av = make_av(_SOURCE)


@register
class TcpRecvwndCommand(CommandDef):
    name = "TCP::recvwnd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::recvwnd",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the receive window size of a TCP connection.",
                synopsis=("TCP::recvwnd ('auto' | WINDOW_SIZE)?",),
                snippet=(
                    "TCP::recvwnd returns the receive window size of a TCP connection.\n"
                    "TCP::recvwnd WINDOW_SIZE sets the receive window to WINDOW_SIZE bytes."
                ),
                source=_SOURCE,
                examples=(
                    "t the receive window size of the TCP flow.\n"
                    "    when CLIENT_ACCEPTED {\n"
                    '        log local0. "TCP set receive window: [TCP::recvwnd 100000]"\n'
                    '        log local0. "TCP get receive window: [TCP::recvwnd]"\n'
                    "    }"
                ),
                return_value="TCP::recvwnd returns the number of bytes that can be stored at the receive window.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::recvwnd ('auto' | WINDOW_SIZE)?",
                    arg_values={
                        0: (
                            _av(
                                "auto", "TCP::recvwnd auto", "TCP::recvwnd ('auto' | WINDOW_SIZE)?"
                            ),
                        )
                    },
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
