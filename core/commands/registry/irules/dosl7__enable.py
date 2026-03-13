# Enriched from F5 iRules reference documentation.
"""DOSL7::enable -- Enables blocking and detection of DoS attacks according to the ASM security policy configuration."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__enable.html"


@register
class Dosl7EnableCommand(CommandDef):
    name = "DOSL7::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables blocking and detection of DoS attacks according to the ASM security policy configuration.",
                synopsis=("DOSL7::enable (DOSL7_PROFILE_OBJ)?",),
                snippet=(
                    "Enables blocking and detection of DoS attacks according to the ASM\n"
                    "security policy configuration. When disabled using DOSL7::disable,\n"
                    "transactions will bypass DoS L7 for both detection and prevention."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    DOSL7::enable\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::enable (DOSL7_PROFILE_OBJ)?",
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
