# Enriched from F5 iRules reference documentation.
"""LSN::pool -- Explicitly set the LSN pool used for translation."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__pool.html"


@register
class LsnPoolCommand(CommandDef):
    name = "LSN::pool"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::pool",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Explicitly set the LSN pool used for translation.",
                synopsis=("LSN::pool LSN_POOL",),
                snippet=(
                    "Explicitly set the LSN pool used for translation.\n\nLSN::pool <pool_name>"
                ),
                source=_SOURCE,
                return_value="LSN::pool",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::pool LSN_POOL",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP", "MR", "RTSP", "SIP"}),
                also_in=frozenset(
                    {"CLIENT_ACCEPTED", "CLIENT_DATA", "LB_FAILED", "LB_SELECTED", "SA_PICKED"}
                ),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
