# Enriched from F5 iRules reference documentation.
"""ISESSION::deduplication -- Allows selection of deduplication based on L7 content inspection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ISESSION__deduplication.html"


@register
class IsessionDeduplicationCommand(CommandDef):
    name = "ISESSION::deduplication"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ISESSION::deduplication",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows selection of deduplication based on L7 content inspection.",
                synopsis=("ISESSION::deduplication BOOL_VALUE",),
                snippet="Allows selection of deduplication based on L7 content inspection",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ISESSION::deduplication BOOL_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
