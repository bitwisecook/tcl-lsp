# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_username -- Returns or sets username and for phishing also additional fields."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_username.html"


@register
class AntifraudAlertUsernameCommand(CommandDef):
    name = "ANTIFRAUD::alert_username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets username and for phishing also additional fields.",
                synopsis=("ANTIFRAUD::alert_username (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_username ;\n"
                    "                Returns username and for phishing also additional fields.\n"
                    "\n"
                    "            ANTIFRAUD::alert_username VALUE ;\n"
                    "                Sets username and for phishing also additional fields."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert username: [ANTIFRAUD::alert_username]."\n'
                    "                ANTIFRAUD::alert_username new_value\n"
                    '                log local0. "new Alert username: [ANTIFRAUD::alert_username]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_username ; Returns username and for phishing also additional fields.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_username (VALUE)?",
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
