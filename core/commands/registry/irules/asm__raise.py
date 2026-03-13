# Enriched from F5 iRules reference documentation.
"""ASM::raise -- Issues a user-defined violation on the request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__raise.html"


@register
class AsmRaiseCommand(CommandDef):
    name = "ASM::raise"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::raise",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Issues a user-defined violation on the request.",
                synopsis=("ASM::raise VIOLATION_NAME (VIOLATION_DETAILS)?",),
                snippet=(
                    "Issues a user-defined violation on the request. The violation\n"
                    "is added to other possible violations, either raised by the ASM or by\n"
                    "previous invocations of this command. The consequent action is\n"
                    "determined by the blocking setting per the raised violation, e.g. if\n"
                    "the violation was set to block, then the request will be blocked."
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    '   if {[ASM::violation count] > 3 and [ASM::severity] eq "Error"} {\n'
                    "      ASM::raise VIOLATION_TOO_MANY_VIOLATIONS\n"
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::raise VIOLATION_NAME (VIOLATION_DETAILS)?",
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
