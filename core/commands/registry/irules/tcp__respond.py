# Enriched from F5 iRules reference documentation.
"""TCP::respond -- Sends the specified data directly to the peer."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__respond.html"


@register
class TcpRespondCommand(CommandDef):
    name = "TCP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends the specified data directly to the peer.",
                synopsis=("TCP::respond RESPONSE_STRING",),
                snippet=(
                    "Sends the specified data directly to the peer. This command can be used\n"
                    "to complete a protocol handshake via an iRule."
                ),
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n    peer { TCP::collect 4 }\n}"),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::respond RESPONSE_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
