# Enriched from F5 iRules reference documentation.
"""LSN::address -- Explicitly set the translation address regardless of the configured LSN pool."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__address.html"


@register
class LsnAddressCommand(CommandDef):
    name = "LSN::address"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::address",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Explicitly set the translation address regardless of the configured LSN pool.",
                synopsis=("LSN::address TRANSLATION_ADDR",),
                snippet=(
                    "Explicitly set the translation address regardless of the configured LSN pool.\n"
                    "\n"
                    "The LSN::address command can be used while processing CLIENT_DATA. This event can occur before and after address translation. If this command is used after translation has occurred an error is thrown.\n"
                    "\n"
                    "Agruments:\n"
                    "    LSN::address - Set the explicit translation IPv4 or IPv6 address for the connection in the current context."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    LSN::address 10.0.0.1\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::address TRANSLATION_ADDR",
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
