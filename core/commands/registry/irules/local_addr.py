# Enriched from F5 iRules reference documentation.
"""local_addr -- Deprecated: Use IP::local_addr instead."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ip__local_addr import IpLocalAddrCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/local_addr.html"


@register
class LocalAddrCommand(CommandDef):
    name = "local_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="local_addr",
            deprecated_replacement=IpLocalAddrCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Use IP::local_addr instead.",
                synopsis=("local_addr",),
                snippet="Deprecated: Use IP::local_addr instead",
                source=_SOURCE,
                examples=(
                    'when CLIENT_ACCEPTED {\n    log local0. "client ip addr: [local_addr]"\n}'
                ),
                return_value="Returns the IP address being used in the connection. In the clientside context, this is the destination IP address from the client request (not necessarily the virtual IP address).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="local_addr",
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
