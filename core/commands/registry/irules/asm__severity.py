# Enriched from F5 iRules reference documentation.
"""ASM::severity -- Returns the overall severity of the violations found in the transaction (both request and response)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__severity.html"


@register
class AsmSeverityCommand(CommandDef):
    name = "ASM::severity"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::severity",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the overall severity of the violations found in the transaction (both request and response).",
                synopsis=("ASM::severity",),
                snippet=(
                    "Returns the overall severity of the violations found in the transaction\n"
                    "(both request and response). This equals to the maximum severity of all\n"
                    "these violations"
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    '   if {[ASM::violation count] > 3 and [ASM::severity] eq "Error"} {\n'
                    "      ASM::raise VIOLATION_TOO_MANY_VIOLATIONS\n"
                    "   }\n"
                    "}"
                ),
                return_value="+ Null string (in case there was no violation until the time the command is invoked) + Emergency + Alert + Critical + Error + Warning + Notice + Informational",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::severity",
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
