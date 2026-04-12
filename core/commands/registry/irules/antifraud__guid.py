# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::guid -- Returns GUID value, only in context of ANTIFRAUD_LOGIN event."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__guid.html"


@register
class AntifraudGuidCommand(CommandDef):
    name = "ANTIFRAUD::guid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::guid",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns GUID value, only in context of ANTIFRAUD_LOGIN event.",
                synopsis=("ANTIFRAUD::guid",),
                snippet="Returns GUID value, only in context of ANTIFRAUD_LOGIN event.",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_LOGIN {\n"
                    '                log local0. "Infected username with GUID [ANTIFRAUD::guid] tried to log in."\n'
                    "            }"
                ),
                return_value="Returns GUID value, only in context of ANTIFRAUD_LOGIN event.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::guid",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ANTIFRAUD"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
