# Enriched from F5 iRules reference documentation.
"""PEM::flow -- PEM iRule command for flow features, including transacitonal and eval."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PEM__flow.html"


@register
class PemFlowCommand(CommandDef):
    name = "PEM::flow"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PEM::flow",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="PEM iRule command for flow features, including transacitonal and eval.",
                synopsis=(
                    "PEM::flow transactional disable",
                    "PEM::flow eval",
                ),
                snippet=(
                    "The transciontal disable command disables the transactional feature in PEM for a flow.\n"
                    "The eval command trigers the policy evaluation for the flow immediately."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    PEM::flow transactional disable;\n"
                    "    PEM::flow eval;\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PEM::flow transactional disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
