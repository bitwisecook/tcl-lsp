# Enriched from F5 iRules reference documentation.
"""client_port -- Returns the TCP port number/service of the specified client."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .tcp__client_port import TcpClientPortCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/client_port.html"


@register
class ClientPortCommand(CommandDef):
    name = "client_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="client_port",
            deprecated_replacement=TcpClientPortCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCP port number/service of the specified client.",
                synopsis=("client_port",),
                snippet="Returns the TCP port number/service of the specified client. This is a BIG-IP version 4.X variable, provided for backward compatibility. You can use the equivalent 9.X command, TCP::client_port instead.",
                source=_SOURCE,
                return_value="client_port Returns the TCP port number/service of the specified client.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="client_port",
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
