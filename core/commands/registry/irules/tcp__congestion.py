# Enriched from F5 iRules reference documentation.
"""TCP::congestion -- Sets the TCP congestion control algorithm."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__congestion.html"


@register
class TcpCongestionCommand(CommandDef):
    name = "TCP::congestion"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::congestion",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the TCP congestion control algorithm.",
                synopsis=("TCP::congestion (none |",),
                snippet="Changes the TCP congestion control algorithm and initializes any state variables unique to the new algorithm.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side congestion control to Woodside.\n"
                    "    clientside {\n"
                    '        log local0. "Client: congestion [TCP::congestion] to woodside"\n'
                    "        TCP::congestion woodside\n"
                    "    }\n"
                    "    # Set server-side congestion control to High-Speed.\n"
                    "    serverside {\n"
                    '        log local0. "Server: congestion [TCP::congestion] to highspeed"\n'
                    "        TCP::congestion highspeed\n"
                    "    }\n"
                    "}"
                ),
                return_value="TCP::congestion returns the current congestion control algorithm.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::congestion (none |",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
