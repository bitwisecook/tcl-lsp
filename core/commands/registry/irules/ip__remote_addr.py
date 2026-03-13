# Enriched from F5 iRules reference documentation.
"""IP::remote_addr -- Returns the IP address of the host on the far end of the connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__remote_addr.html"


_av = make_av(_SOURCE)


@register
class IpRemoteAddrCommand(CommandDef):
    name = "IP::remote_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::remote_addr",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the IP address of the host on the far end of the connection.",
                synopsis=("IP::remote_addr (clientside | serverside)?",),
                snippet=(
                    "Returns the IP address of the host on the far end of the connection. In the clientside context, this is the client IP address. In the serverside context this is the node IP address. You can also specify the IP::client_addr and IP::server_addr commands, respectively.\n"
                    "\n"
                    "In BIG-IP 10.x with route domains enabled this command returns the remote IP address in the x.x.x.x%rd of the server or client (depending on the context) that is in any non-default route domain else it returns just the IP address as expected.\n"
                    "\n"
                    "This command is equivalent to the BIG-IP 4.X variable remote_addr."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '   log local0. "Node IP address is: [IP::remote_addr]"\n'
                    "}"
                ),
                return_value="IP address of the host on the far end of the connection",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::remote_addr (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "IP::remote_addr clientside",
                                "IP::remote_addr (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "IP::remote_addr serverside",
                                "IP::remote_addr (clientside | serverside)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.CONNECTION,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={None: TaintColour.TAINTED | TaintColour.IP_ADDRESS},
        )
