# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::enable -- Enables the anti-fraud plugin."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__enable.html"


@register
class AntifraudEnableCommand(CommandDef):
    name = "ANTIFRAUD::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables the anti-fraud plugin.",
                synopsis=("ANTIFRAUD::enable (ANTIFRAUD_PROFILE)?",),
                snippet="Enables the anti-fraud plugin.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "                # apply default anti-fraud profile on the transaction with Antifraud-Foo HTTP header\n"
                    '                if { [HTTP::header exists "Antifraud-Foo" ] } {\n'
                    "                    ANTIFRAUD::enable\n"
                    "                }\n"
                    "                # apply /Common/antifraud_bar profile on the transaction with Antifraud-Bar HTTP header\n"
                    '                if { [HTTP::header exists "Antifraud-Bar" ] } {\n'
                    "                    ANTIFRAUD::enable /Common/antifraud_bar\n"
                    "                }\n"
                    "            }"
                ),
                return_value="ANTIFRAUD::enable Applies the default anti-fraud profile attached to the virtual server.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::enable (ANTIFRAUD_PROFILE)?",
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
