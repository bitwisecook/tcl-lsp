# Enriched from F5 iRules reference documentation.
"""TAP::insight_requested -- TAP requests insight flag."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TAP__insight_requested.html"


@register
class TapInsightRequestedCommand(CommandDef):
    name = "TAP::insight_requested"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TAP::insight_requested",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="TAP requests insight flag.",
                synopsis=("TAP::insight_requested",),
                snippet="Command for indication that TAP module wants to get insight from any module. TAP::insight must be invoked if flag is set to true.",
                source=_SOURCE,
                return_value="Return boolean value if TAP modules wants to get an insight from any module.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TAP::insight_requested",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
