# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_license_id -- Returns crc32 of the license id in hex."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_license_id.html"


@register
class AntifraudAlertLicenseIdCommand(CommandDef):
    name = "ANTIFRAUD::alert_license_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_license_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns crc32 of the license id in hex.",
                synopsis=("ANTIFRAUD::alert_license_id",),
                snippet="Returns crc32 of the license id in hex.",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "Alert license ID: [ANTIFRAUD::alert_license_id]."\n'
                    "            }"
                ),
                return_value="Returns crc32 of the license id in hex.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_license_id",
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
