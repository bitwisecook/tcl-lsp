# Enriched from F5 iRules reference documentation.
"""TCP::limxmit -- Toggles the TCP limited transmit."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__limxmit.html"


@register
class TcpLimxmitCommand(CommandDef):
    name = "TCP::limxmit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::limxmit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles the TCP limited transmit.",
                synopsis=("TCP::limxmit BOOL_VALUE",),
                snippet="Enables or disables TCP limited transmit recovery, which sends new data in response to duplicate acks. See RFC3042 for details.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set server-side limited transmit to disable.\n"
                    "    TCP::limxmit disable\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::limxmit BOOL_VALUE",
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
