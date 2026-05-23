# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::disable -- Deprecated: Disables classification for the current flow."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__disabled.html"


@register
class ClassificationDisableCommand(CommandDef):
    name = "CLASSIFICATION::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Disables classification for the current flow.",
                synopsis=("CLASSIFICATION::disable",),
                snippet=(
                    "This command disables classification for the current flow.\n"
                    "\n"
                    "CLASSIFICATION::disable"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
