# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::username -- Provides username associated with classification results."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__username.html"


@register
class ClassificationUsernameCommand(CommandDef):
    name = "CLASSIFICATION::username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides username associated with classification results.",
                synopsis=("CLASSIFICATION::username",),
                snippet="CLASSIFICATION::username",
                source=_SOURCE,
                examples=(
                    "when CLASSIFICATION_DETECTED {\n"
                    "        set res [CLASSIFICATION::username]\n"
                    '        log local0.debug "DPI username: $res"\n'
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::username",
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
