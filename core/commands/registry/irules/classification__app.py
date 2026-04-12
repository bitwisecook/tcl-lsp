# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::app -- Deprecated: Provides classification for the most explicit application name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__app.html"


@register
class ClassificationAppCommand(CommandDef):
    name = "CLASSIFICATION::app"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::app",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Provides classification for the most explicit application name.",
                synopsis=("CLASSIFICATION::app",),
                snippet=(
                    "This command provides classification for the most explicit application\n"
                    "name. (Example: cnn, amazon)\n"
                    "\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work.\n"
                    "\n"
                    "CLASSIFICATION::app"
                ),
                source=_SOURCE,
                examples=(
                    "when CLASSIFICATION_DETECTED {\n"
                    '  if { [CLASSIFICATION::app] equals "application1"}  {\n'
                    "    drop\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::app",
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
