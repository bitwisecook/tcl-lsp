# Enriched from F5 iRules reference documentation.
"""IP::local_addr -- Returns the IP address of the virtual server the client is connected to or the self-ip LTM is connected from."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__local_addr.html"


_av = make_av(_SOURCE)


@register
class IpLocalAddrCommand(CommandDef):
    name = "IP::local_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::local_addr",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the IP address of the virtual server the client is connected to or the self-ip LTM is connected from.",
                synopsis=("IP::local_addr (clientside | serverside)?",),
                snippet=(
                    "When called in a clientside context, this command returns the IP address of the virtual server the client is connected to. When called in a serverside context it returns the self-ip address or spoofed client IP address LTM is using for the serverside connection.\n"
                    "\n"
                    "This command is primarily useful for generic rules that are re-used. Also, it is useful in reusing the connected endpoint in another statement (such as with the listen command) or to make routing type decisions. You can also specify the IP::client_addr and IP::server_addr commands.\n"
                    "\n"
                    "This command in BIG-IP 10."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '   log local0. "Source IP address for connection to node: [IP::local_addr]"\n'
                    "}"
                ),
                return_value="Returns the IP address being used in the connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::local_addr (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "IP::local_addr clientside",
                                "IP::local_addr (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "IP::local_addr serverside",
                                "IP::local_addr (clientside | serverside)?",
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
