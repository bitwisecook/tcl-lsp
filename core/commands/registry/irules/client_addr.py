# Enriched from F5 iRules reference documentation.
"""client_addr -- Returns the client IP address of a connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ip__client_addr import IpClientAddrCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/client_addr.html"


@register
class ClientAddrCommand(CommandDef):
    name = "client_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="client_addr",
            deprecated_replacement=IpClientAddrCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the client IP address of a connection.",
                synopsis=("client_addr",),
                snippet="Returns the client IP address of a connection. This is a BIG-IP version 4.X variable, provided for backward compatibility. You can use the equivalent 9.X command, IP::client_addr instead.",
                source=_SOURCE,
                return_value="client_addr Returns the client IP address of a connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="client_addr",
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
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
