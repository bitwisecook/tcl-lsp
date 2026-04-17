# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::result -- Returns result of login validation (passed or failed)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__result.html"


@register
class AntifraudResultCommand(CommandDef):
    name = "ANTIFRAUD::result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns result of login validation (passed or failed).",
                synopsis=("ANTIFRAUD::result",),
                snippet="Returns result of login validation (passed or failed).",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_LOGIN {\n"
                    '                log local0. "Username tried to log in with result [ANTIFRAUD::result]."\n'
                    "            }"
                ),
                return_value="Returns result of login validation (passed or failed).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::result",
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
