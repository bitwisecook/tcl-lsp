# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::fingerprint -- Returns fingerprint data, only in context of ANTIFRAUD_LOGIN event."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__fingerprint.html"


@register
class AntifraudFingerprintCommand(CommandDef):
    name = "ANTIFRAUD::fingerprint"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::fingerprint",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns fingerprint data, only in context of ANTIFRAUD_LOGIN event.",
                synopsis=("ANTIFRAUD::fingerprint",),
                snippet="Returns fingerprint data collected on client side.",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_LOGIN {\n"
                    '                log local0. "Client fingerprint: [ANTIFRAUD::fingerprint]."\n'
                    "            }"
                ),
                return_value="Returns fingerprint data collected on client side.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::fingerprint",
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
