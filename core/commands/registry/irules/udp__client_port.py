# Enriched from F5 iRules reference documentation.
"""UDP::client_port -- Returns the UDP port/service number of a client system."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__client_port.html"


@register
class UdpClientPortCommand(CommandDef):
    name = "UDP::client_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::client_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the UDP port/service number of a client system.",
                synopsis=("UDP::client_port",),
                snippet=(
                    "Returns the UDP port/service number of the client system. This command\n"
                    "is equivalent to the command clientside { UDP::remote_port }."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    '  if { [UDP::payload 50] contains "XYZ" } {\n'
                    "    pool xyz_servers\n"
                    '    persist uie "[IP::client_addr]:[UDP::client_port]" 300\n'
                    "  } else {\n"
                    "    pool web_servers\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the UDP port/service number of the client system",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::client_port",
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
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
