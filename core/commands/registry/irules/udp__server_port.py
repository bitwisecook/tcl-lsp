# Enriched from F5 iRules reference documentation.
"""UDP::server_port -- Returns the UDP port/service number of a server system."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__server_port.html"


@register
class UdpServerPortCommand(CommandDef):
    name = "UDP::server_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::server_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the UDP port/service number of a server system.",
                synopsis=("UDP::server_port",),
                snippet=(
                    "Returns the UDP port/service number of the server. This command is\n"
                    "equivalent to the command serverside { UDP::remote_port }."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    set client [IP::client_addr]:[UDP::client_port]\n"
                    "    set node [IP::server_addr]:[UDP::server_port]\n"
                    '    log local0. "client: $client, server: $server"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::server_port",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="udp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
