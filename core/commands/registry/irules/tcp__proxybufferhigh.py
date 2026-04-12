# Enriched from F5 iRules reference documentation.
"""TCP::proxybufferhigh -- Gets proxy buffer high threshold."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__proxybufferhigh.html"


@register
class TcpProxybufferhighCommand(CommandDef):
    name = "TCP::proxybufferhigh"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::proxybufferhigh",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets proxy buffer high threshold.",
                synopsis=("TCP::proxybufferhigh",),
                snippet="Gets the threshold at which the proxy buffer stops accepting new data, in bytes.",
                source=_SOURCE,
                examples=(
                    'when SERVER_CONNECTED {\n    log local0.debug "[TCP::proxybufferhigh]"\n}'
                ),
                return_value="The proxy buffer high threshold.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::proxybufferhigh",
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
