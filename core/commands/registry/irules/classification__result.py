# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::result -- Provides classification results."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__result.html"


@register
class ClassificationResultCommand(CommandDef):
    name = "CLASSIFICATION::result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides classification results.",
                synopsis=("CLASSIFICATION::result",),
                snippet="CLASSIFICATION::result",
                source=_SOURCE,
                examples=(
                    "when CLASSIFICATION_DETECTED {\n"
                    "        set res [CLASSIFICATION::result]\n"
                    '        log local0.debug "DPI results: $res"\n'
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::result",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CLASSIFICATION"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
