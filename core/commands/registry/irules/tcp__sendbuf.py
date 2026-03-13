# Enriched from F5 iRules reference documentation.
"""TCP::sendbuf -- This command can be used to set/get the send buffer size of a TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__sendbuf.html"


_av = make_av(_SOURCE)


@register
class TcpSendbufCommand(CommandDef):
    name = "TCP::sendbuf"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::sendbuf",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the send buffer size of a TCP connection.",
                synopsis=("TCP::sendbuf ('auto' | BUFFER_SIZE)?",),
                snippet=(
                    "TCP::sendbuf returns the send buffer size of a TCP connection.\n"
                    "TCP::sendbuf BUFFER_SIZE sets the send buffer size to BUFFER_SIZE bytes."
                ),
                source=_SOURCE,
                examples=(
                    "t the send buffer size of the TCP flow.\n"
                    "    when CLIENT_ACCEPTED {\n"
                    '        log local0. "TCP set send buffer: [TCP::sendbuf 100000]"\n'
                    '        log local0. "TCP get send buffer: [TCP::sendbuf]"\n'
                    "    }"
                ),
                return_value="TCP::sendbuf returns the number of bytes that can be stored at the send buffer.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::sendbuf ('auto' | BUFFER_SIZE)?",
                    arg_values={
                        0: (
                            _av(
                                "auto", "TCP::sendbuf auto", "TCP::sendbuf ('auto' | BUFFER_SIZE)?"
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
