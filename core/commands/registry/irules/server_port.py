# Enriched from F5 iRules reference documentation.
"""server_port -- Returns the TCP port/service number of the specified server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .tcp__server_port import TcpServerPortCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/server_port.html"


@register
class ServerPortCommand(CommandDef):
    name = "server_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="server_port",
            deprecated_replacement=TcpServerPortCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCP port/service number of the specified server.",
                synopsis=("server_port",),
                snippet=(
                    "Returns the TCP port/service number of the specified server. This is a\n"
                    "BIG-IP version 4.X variable, provided for backward compatibility. You\n"
                    "can use the equivalent 9.X command TCP::server_port instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="server_port",
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
