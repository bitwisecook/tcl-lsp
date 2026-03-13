# Enriched from F5 iRules reference documentation.
"""TCP::autowin -- Toggles automatic window tuning."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__autowin.html"


@register
class TcpAutowinCommand(CommandDef):
    name = "TCP::autowin"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::autowin",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles automatic window tuning.",
                synopsis=("TCP::autowin BOOL_VALUE",),
                snippet="Sets the send and receive buffer dynamically in accordance with measured connection parameters.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    # Enable auto buffer tuning on HTTP request(s).\n"
                    '    log local0. "Send buffer: [TCP::sendbuf] Receive Window: [TCP::recvwnd]"\n'
                    '    log local0. "HTTP request, auto buffer tuning enabled."\n'
                    "    TCP::autowin enable\n"
                    '    log local0. "Send buffer: [TCP::sendbuf] Receive Window: [TCP::recvwnd]"\n'
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::autowin BOOL_VALUE",
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
