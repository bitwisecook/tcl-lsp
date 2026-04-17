# Enriched from F5 iRules reference documentation.
"""PROTOCOL_INSPECTION::disable -- Disables inspection match of the flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROTOCOL_INSPECTION__disable.html"


@register
class ProtocolInspectionDisableCommand(CommandDef):
    name = "PROTOCOL_INSPECTION::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROTOCOL_INSPECTION::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables inspection match of the flow.",
                synopsis=("PROTOCOL_INSPECTION::disable",),
                snippet="Disables inspection of the flow",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROTOCOL_INSPECTION::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
