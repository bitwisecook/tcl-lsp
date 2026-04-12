# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::username -- Returns or sets username value, only in context of ANTIFRAUD_LOGIN event."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__username.html"


@register
class AntifraudUsernameCommand(CommandDef):
    name = "ANTIFRAUD::username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets username value, only in context of ANTIFRAUD_LOGIN event.",
                synopsis=("ANTIFRAUD::username (USERNAME_ALIAS)?",),
                snippet=(
                    "ANTIFRAUD::username\n"
                    "                Returns username value, only in context of ANTIFRAUD_LOGIN event.\n"
                    "\n"
                    "            ANTIFRAUD::username USERNAME_ALIAS ;\n"
                    "                Replaces existing username with USERNAME_ALIAS, only in context of ANTIFRAUD_LOGIN event."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_LOGIN {\n"
                    '                log local0. "Username [ANTIFRAUD::username] tried to log in."\n'
                    "                ANTIFRAUD::username username_alias\n"
                    '                log local0. "Username alias is [ANTIFRAUD::username]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::username Returns username value, only in context of ANTIFRAUD_LOGIN event.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::username (USERNAME_ALIAS)?",
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
