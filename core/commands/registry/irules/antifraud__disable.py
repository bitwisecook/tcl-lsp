# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::disable -- Disables the anti-fraud plugin."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable.html"


@register
class AntifraudDisableCommand(CommandDef):
    name = "ANTIFRAUD::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables the anti-fraud plugin.",
                synopsis=("ANTIFRAUD::disable",),
                snippet="Disables the anti-fraud plugin.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "                # Disable request with Antifraud-Disable header (bypass antifraud plugin)\n"
                    '                if { [HTTP::header exists "Antifraud-Disable" ] } {\n'
                    "                    ANTIFRAUD::disable\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
