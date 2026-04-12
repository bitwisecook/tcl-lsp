# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::client_id -- Returns client id collected on client side."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__client_id.html"


@register
class AntifraudClientIdCommand(CommandDef):
    name = "ANTIFRAUD::client_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::client_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns client id collected on client side.",
                synopsis=("ANTIFRAUD::client_id",),
                snippet="Returns client id collected on client side.",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "client id: [ANTIFRAUD::client_id]."\n'
                    "            }"
                ),
                return_value="Returns client id collected on client side.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::client_id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
