# Enriched from F5 iRules reference documentation.
"""ASM::microservice -- request matched microservice"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__microservice.html"


@register
class AsmMicroserviceCommand(CommandDef):
    name = "ASM::microservice"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::microservice",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="request matched microservice",
                synopsis=("ASM::microservice",),
                snippet="returns the microservice matched for the request;",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE \n"
                    "            {\n"
                    '		if {[ASM::microservice] eq "*a/login.php"}\n'
                    "		{\n"
                    '			log local0. "Microservice : found"\n'
                    "	        }\n"
                    "            }"
                ),
                return_value="returns the microservice matched for the request;",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::microservice",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
