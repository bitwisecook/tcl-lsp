# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::urlcat -- Deprecated: provides classification url category name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__urlcat.html"


@register
class ClassificationUrlcatCommand(CommandDef):
    name = "CLASSIFICATION::urlcat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::urlcat",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: provides classification url category name.",
                synopsis=("CLASSIFICATION::urlcat",),
                snippet=(
                    "This command provides classification url category name.\n"
                    "\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work.\n"
                    "\n"
                    "CLASSIFICATION::urlcat"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::urlcat",
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
