# Enriched from F5 iRules reference documentation.
"""IP::client_addr -- Returns the client IP address of a connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__client_addr.html"


@register
class IpClientAddrCommand(CommandDef):
    name = "IP::client_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::client_addr",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the client IP address of a connection.",
                synopsis=("IP::client_addr",),
                snippet="Returns the client IP address of a connection. This command is equivalent to the command clientside { IP::remote_addr } and to the BIG-IP 4.X variable client_addr.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [IP::addr [IP::client_addr] equals 10.10.10.10] } {\n"
                    "     pool my_pool\n"
                    " }\n"
                    "}"
                ),
                return_value="In BIG-IP 10.x with route domains enabled if the client is in any non-default route domain, this command returns the client IP address in the x.x.x.x%rd. For clients in the default route domain, it returns just the IPv4 address.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::client_addr",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True),
            cse_candidate=True,
            xc_translatable=True,
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
            source={Arity(0, 0): TaintColour.TAINTED | TaintColour.IP_ADDRESS},
        )
