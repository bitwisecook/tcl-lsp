# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::enable -- Deprecated: Enables classification for the current flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__enabled.html"


@register
class ClassificationEnableCommand(CommandDef):
    name = "CLASSIFICATION::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Enables classification for the current flow.",
                synopsis=("CLASSIFICATION::enable",),
                snippet=(
                    "This command enables classification for the current flow.\n"
                    "\n"
                    "CLASSIFICATION::enable"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::enable",
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
