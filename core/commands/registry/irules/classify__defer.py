# Enriched from F5 iRules reference documentation.
"""CLASSIFY::defer -- Defers the classification of the flow to response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFY__defer.html"


@register
class ClassifyDeferCommand(CommandDef):
    name = "CLASSIFY::defer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFY::defer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Defers the classification of the flow to response.",
                synopsis=("CLASSIFY::defer",),
                snippet=(
                    "This command defers the classification of the flow to response (for\n"
                    "HTTP - response message, for non-HTTP, opposite traffic direction from\n"
                    "traffic initiator)\n"
                    "\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work.\n"
                    "\n"
                    "CLASSIFY::defer"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFY::defer",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
