# Enriched from F5 iRules reference documentation.
"""LSN::inbound -- Disable inbound mapping for translation address and port associated with the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__inbound.html"


@register
class LsnInboundCommand(CommandDef):
    name = "LSN::inbound"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::inbound",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disable inbound mapping for translation address and port associated with the current connection.",
                synopsis=("LSN::inbound disable",),
                snippet="Disable inbound mapping for translation address and port associated with the current connection.",
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    LSN::inbound disable\n}"),
                return_value="LSN::inbound disable - Inbound connections can be permitted for a particular LSN pool to provide end-point independent filtering, described in RFC 4787.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::inbound disable",
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
