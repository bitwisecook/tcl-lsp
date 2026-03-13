# Enriched from F5 iRules reference documentation.
"""TCP::unused_port -- Returns an unused TCP port for the specified IP tuple."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__unused_port.html"


@register
class TcpUnusedPortCommand(CommandDef):
    name = "TCP::unused_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::unused_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an unused TCP port for the specified IP tuple.",
                synopsis=("TCP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR",),
                snippet=(
                    "Returns an unused TCP port for the specified IP tuple, using the value\n"
                    "of <hint_port> as a starting point if it is supplied. If no appropriate\n"
                    "unused local port could be found, 0 is returned."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    log local0. [TCP::unused_port 192.168.1.124 80 192.168.1.146]\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
