# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_fingerprint -- Returns or sets fingerprint data collected on client side."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_fingerprint.html"


@register
class AntifraudAlertFingerprintCommand(CommandDef):
    name = "ANTIFRAUD::alert_fingerprint"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_fingerprint",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets fingerprint data collected on client side.",
                synopsis=("ANTIFRAUD::alert_fingerprint (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_fingerprint ;\n"
                    "                Returns fingerprint data collected on client side.\n"
                    "\n"
                    "            ANTIFRAUD::alert_fingerprint VALUE ;\n"
                    "                Sets fingerprint data collected on client side."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert fingerprint: [ANTIFRAUD::alert_fingerprint]."\n'
                    "                ANTIFRAUD::alert_fingerprint new_value\n"
                    '                log local0. "new Alert fingerprint: [ANTIFRAUD::alert_fingerprint]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_fingerprint ; Returns fingerprint data collected on client side.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_fingerprint (VALUE)?",
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
