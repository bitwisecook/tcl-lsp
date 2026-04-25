# Enriched from F5 iRules reference documentation.
"""TCP::idletime -- Sets the TCP Idle Timeout."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__idletime.html"


@register
class TcpIdletimeCommand(CommandDef):
    name = "TCP::idletime"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::idletime",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the TCP Idle Timeout.",
                synopsis=("TCP::idletime IDLE_TIME",),
                snippet="Sets the number of seconds before BIG-IP deletes connections with no traffic. A value of zero indicates no time limit.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set server-side idletime to 100.\n"
                    "    TCP::idletime 100\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::idletime IDLE_TIME",
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
