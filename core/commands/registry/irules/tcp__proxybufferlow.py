# Enriched from F5 iRules reference documentation.
"""TCP::proxybufferlow -- Gets proxy buffer low threshold."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__proxybufferlow.html"


@register
class TcpProxybufferlowCommand(CommandDef):
    name = "TCP::proxybufferlow"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::proxybufferlow",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets proxy buffer low threshold.",
                synopsis=("TCP::proxybufferlow",),
                snippet="Gets the threshold at which the proxy buffer starts sending new data, in bytes.",
                source=_SOURCE,
                examples=(
                    'when SERVER_CONNECTED {\n    log local0.debug "[TCP::proxybufferlow]"\n}'
                ),
                return_value="The proxy buffer low threshold.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::proxybufferlow",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(server_side=True, transport="tcp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
