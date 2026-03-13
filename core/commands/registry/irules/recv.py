# Enriched from F5 iRules reference documentation.
"""recv -- Receives data from a given sideband connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/recv.html"


@register
class RecvCommand(CommandDef):
    name = "recv"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="recv",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Receives data from a given sideband connection.",
                synopsis=("recv ((",),
                snippet="This command receives data from an existing sideband connection (established with connect). It is one of several commands that make up the ability to create sideband connections from iRules.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    set dest "10.10.145.1:80"\n'
                    "    set conn [connect -protocol TCP -timeout 30000 -idle 30 $dest]\n"
                    '    set data "GET /mypage/index.html HTTP/1.0\\r\\n\\r\\n"\n'
                    "    send $conn $data\n"
                    "    # recv 2000 chars get recv bytes error\n"
                    "    set recv_2kbytes [recv -timeout 30000 2000 $conn]\n"
                    "    log local0. \"Recv data: '$recv_2kbytes'\"\n"
                    "    set length_recv [string length $recv_2kbytes]\n"
                    '    log local0. "length_recv has $length_recv bytes in it."\n'
                    "    close $conn\n"
                    "}"
                ),
                return_value="Receives some data (up to numChars bytes) from a sideband connection. If varname is specified, the response is stored in that variable, and recv returns the amount of data received. Otherwise, recv returns the response data.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="recv ((",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
