# Enriched from F5 iRules reference documentation.
"""LSN::port -- Explicitly set the translation address regardless of the configured LSN pool."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__port.html"


@register
class LsnPortCommand(CommandDef):
    name = "LSN::port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Explicitly set the translation address regardless of the configured LSN pool.",
                synopsis=("LSN::port TRANSLATION_PORT",),
                snippet=(
                    "Explicitly set the translation address regardless of the configured LSN pool.\n"
                    "\n"
                    "The LSN::port command can be used while processing CLIENT_DATA. This event can occur before and after address translation. If this command is used after translation has occurred an error is thrown.\n"
                    "\n"
                    "LSN::port <translation_port>"
                ),
                source=_SOURCE,
                return_value="LSN::port",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::port TRANSLATION_PORT",
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
