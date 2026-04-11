# Enriched from F5 iRules reference documentation.
"""LSN::disable -- Disables LSN translation for the current connection if LSN translation has been configured."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__disable.html"


@register
class LsnDisableCommand(CommandDef):
    name = "LSN::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables LSN translation for the current connection if LSN translation has been configured.",
                synopsis=("LSN::disable",),
                snippet=(
                    "Disables LSN translation for the current connection if LSN translation has been configured.\n"
                    "\n"
                    "Arguments:\n"
                    "    LSN::disable - If LSN translation is configured, disables translation for this connection."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    LSN::disable\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=_LSN_EVENT_REQUIRES,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
