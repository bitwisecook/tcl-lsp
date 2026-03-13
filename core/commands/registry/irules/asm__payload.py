# Enriched from F5 iRules reference documentation.
"""ASM::payload -- Retrieves or replaces the payload collected by ASM."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__payload.html"


@register
class AsmPayloadCommand(CommandDef):
    name = "ASM::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Retrieves or replaces the payload collected by ASM.",
                synopsis=(
                    "ASM::payload (LENGTH | (OFFSET LENGTH))?",
                    "ASM::payload length",
                    "ASM::payload replace OFFSET LENGTH ASM_PAYLOAD",
                ),
                snippet="This command retrieves or replaces the payload collected by ASM.",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_VIOLATION\n"
                    "{\n"
                    "  set x [ASM::violation_data]\n"
                    '  if {([lindex $x 0] contains "VIOLATION_EVASION_DETECTED")}\n'
                    "   {\n"
                    '      ASM::payload replace 0 0 "1234567890"\n'
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::payload (LENGTH | (OFFSET LENGTH))?",
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
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
