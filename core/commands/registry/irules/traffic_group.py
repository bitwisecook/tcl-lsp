# Enriched from F5 iRules reference documentation.
"""traffic_group -- Returns the current traffic group."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/traffic_group.html"


@register
class TrafficGroupCommand(CommandDef):
    name = "traffic_group"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="traffic_group",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current traffic group.",
                synopsis=("traffic_group",),
                snippet=(
                    "This command returns the current traffic group. Useful for\n"
                    "troubleshooting the partitioning of data in the session table."
                ),
                source=_SOURCE,
                examples=(
                    'when RULE_INIT {\n  log local0. "traffic_group name: [traffic_group]"\n}'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="traffic_group",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
