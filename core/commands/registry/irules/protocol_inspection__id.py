# Enriched from F5 iRules reference documentation.
"""PROTOCOL_INSPECTION::id -- Provides protocol inspection match result."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROTOCOL_INSPECTION__id.html"


@register
class ProtocolInspectionIdCommand(CommandDef):
    name = "PROTOCOL_INSPECTION::id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROTOCOL_INSPECTION::id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides protocol inspection match result.",
                synopsis=("PROTOCOL_INSPECTION::id",),
                snippet="This command provides inspection match result.",
                source=_SOURCE,
                examples=(
                    "when PROTOCOL_INSPECTION_MATCH {\n"
                    "    set id [PROTOCOL_INSPECTION::id]\n"
                    '    log local0.debug "inspection id: $id"\n'
                    "}"
                ),
                return_value="PROTOCOL_INSPECTION::id returns inspection id array",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROTOCOL_INSPECTION::id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"PROTOCOL_INSPECTION"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
