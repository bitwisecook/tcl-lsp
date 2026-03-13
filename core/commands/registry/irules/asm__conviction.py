# Enriched from F5 iRules reference documentation.
"""ASM::conviction -- Inject conviction honey traps in case of behavioral enforcement is enabled"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__conviction.html"


@register
class AsmConvictionCommand(CommandDef):
    name = "ASM::conviction"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::conviction",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Inject conviction honey traps in case of behavioral enforcement is enabled",
                synopsis=("ASM::conviction",),
                snippet="Inject conviction honey traps in case of behavioral enforcement is enabled",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE\n"
                    "            {\n"
                    "                ASM::conviction\n"
                    "            }"
                ),
                return_value="no return value",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::conviction",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
