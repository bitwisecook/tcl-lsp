# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::category -- Deprecated: Provides classification category name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__category.html"


@register
class ClassificationCategoryCommand(CommandDef):
    name = "CLASSIFICATION::category"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::category",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Provides classification category name.",
                synopsis=("CLASSIFICATION::category",),
                snippet=(
                    "This command provides classification category name. (Example: mail,\n"
                    "gaming)\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work."
                ),
                source=_SOURCE,
                examples=(
                    "when CLASSIFICATION_DETECTED {\n"
                    '  if { [CLASSIFICATION::category] equals "chat"}  {\n'
                    "    drop\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::category",
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
