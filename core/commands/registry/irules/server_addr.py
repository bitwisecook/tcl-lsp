# Enriched from F5 iRules reference documentation.
"""server_addr -- Returns the IP address of the server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ip__server_addr import IpServerAddrCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/server_addr.html"


@register
class ServerAddrCommand(CommandDef):
    name = "server_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="server_addr",
            deprecated_replacement=IpServerAddrCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the IP address of the server.",
                synopsis=("server_addr",),
                snippet=(
                    "Returns the IP address of the server. This is a BIG-IP version 4.X\n"
                    "variable, provided for backward compatibility. You can use the\n"
                    "equivalent 9.X command IP::server_addr instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="server_addr",
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
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
