# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::disable_alert -- Disables the current alert."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_alert.html"


@register
class AntifraudDisableAlertCommand(CommandDef):
    name = "ANTIFRAUD::disable_alert"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::disable_alert",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables the current alert.",
                synopsis=("ANTIFRAUD::disable_alert",),
                snippet="Disables the current alert",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    "                ANTIFRAUD::disable_alert\n"
                    "            }"
                ),
                return_value="Disables the current alert",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::disable_alert",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ANTIFRAUD"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
