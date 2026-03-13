# Enriched from F5 iRules reference documentation.
"""DOSL7::disable -- Disables blocking and detection of DoS attacks according to the ASM security policy configuration."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__disable.html"


@register
class Dosl7DisableCommand(CommandDef):
    name = "DOSL7::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables blocking and detection of DoS attacks according to the ASM security policy configuration.",
                synopsis=("DOSL7::disable",),
                snippet=(
                    "Disables blocking and detection of DoS attacks according to the ASM\n"
                    "security policy configuration. When enabled using DOSL7::enable,\n"
                    "transactions will be enforced according to the DoS L7 ASM policy\n"
                    "configuration for both detection and prevention."
                ),
                source=_SOURCE,
                examples=("when IN_DOSL7_ATTACK {\n    DOSL7::disable\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DOSL7_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
