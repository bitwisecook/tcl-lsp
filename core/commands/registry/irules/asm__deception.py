# Enriched from F5 iRules reference documentation.
"""ASM::deception -- Mark a request as deceptive for further enforcement by asm"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__deception.html"


@register
class AsmDeceptionCommand(CommandDef):
    name = "ASM::deception"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::deception",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Mark a request as deceptive for further enforcement by asm",
                synopsis=("ASM::deception",),
                snippet="Mark a request as deceptive for further enforcement by asm",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE\n"
                    "            {\n"
                    "                ASM::deception\n"
                    "            }"
                ),
                return_value="no return value",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::deception",
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
