# Enriched from F5 iRules reference documentation.
"""TCP::ecn -- Toggles TCP Explicit Congestion Notification."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__ecn.html"


@register
class TcpEcnCommand(CommandDef):
    name = "TCP::ecn"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::ecn",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles TCP Explicit Congestion Notification.",
                synopsis=("TCP::ecn BOOL_VALUE",),
                snippet=(
                    "Enables or disables TCP explicit congestion notification.\n"
                    "When enabled, respond to explicit router notification of congestion by invoking the TCP congestion response.\n"
                    "See RFC3168 for details."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    TCP::ecn disable\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::ecn BOOL_VALUE",
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
