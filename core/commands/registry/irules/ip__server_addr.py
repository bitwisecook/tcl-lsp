# Enriched from F5 iRules reference documentation.
"""IP::server_addr -- Returns the server's IP address."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__server_addr.html"


@register
class IpServerAddrCommand(CommandDef):
    name = "IP::server_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::server_addr",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the server's IP address.",
                synopsis=("IP::server_addr",),
                snippet=(
                    "Returns the server's (node's) IP address once a serverside connection has been established. This command is equivalent to the command serverside { IP::remote_addr } and to the BIG-IP 4.X variable server_addr. The command returns 0 if the serverside connection has not been made.\n"
                    "\n"
                    "In BIG-IP 10.x with route domains enabled this command returns the server's (node's) address once the serverside connection is established in the x.x.x.x%rd if the server is in any non-default route domains else it returns just the IPv4 address as expected."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '   log local0. "Node IP address: [IP::server_addr]"\n'
                    "}"
                ),
                return_value="server's IP address",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::server_addr",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(server_side=True),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
