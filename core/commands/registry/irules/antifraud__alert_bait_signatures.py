# Generated from F5 iRules reference documentation -- do not edit manually.
"""ANTIFRAUD::alert_bait_signatures -- Deprecated: For the trojan_bait alert: returns the bait signatures in an escaped base64 format."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .antifraud__alert_details import AntifraudAlertDetailsCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_bait_signatures.html"


@register
class AntifraudAlertBaitSignaturesCommand(CommandDef):
    name = "ANTIFRAUD::alert_bait_signatures"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_bait_signatures",
            deprecated_replacement=AntifraudAlertDetailsCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: For the trojan_bait alert: returns the bait signatures in an escaped base64 format.",
                synopsis=("ANTIFRAUD::alert_bait_signatures",),
                snippet="For the trojan_bait alert: returns the bait signatures in an escaped base64 format.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_bait_signatures",
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
