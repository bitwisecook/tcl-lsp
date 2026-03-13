# Enriched from F5 iRules reference documentation.
"""remote_addr -- Deprecated: Use IP::remote_addr instead."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ip__remote_addr import IpRemoteAddrCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/remote_addr.html"


@register
class RemoteAddrCommand(CommandDef):
    name = "remote_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="remote_addr",
            deprecated_replacement=IpRemoteAddrCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Use IP::remote_addr instead.",
                synopsis=("remote_addr",),
                snippet=(
                    "Returns the IP address of the host on the far end of the connection. In the clientside context, this is the client IP address. In the serverside context this is the node IP address. You can also specify the IP::client_addr and IP::server_addr commands, respectively.\n"
                    "\n"
                    "In BIG-IP 10.x with route domains enabled this command returns the remote IP address in the x.x.x.x%rd of the server or client (depending on the context) that is in any non-default route domain else it returns just the IP address as expected.\n"
                    "\n"
                    "This command is equivalent to the BIG-IP 4.X variable remote_addr."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { [IP::addr [IP::remote_addr] equals 206.0.0.0 mask 255.0.0.0] } {\n"
                    "        pool clients_from_206\n"
                    "    } else {\n"
                    "        pool other_clients_pool\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the IP address of the host on the far end of the connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="remote_addr",
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
