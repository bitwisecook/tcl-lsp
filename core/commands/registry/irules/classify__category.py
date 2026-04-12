# Enriched from F5 iRules reference documentation.
"""CLASSIFY::category -- Allows to set or add a category name to the classification."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFY__category.html"


_av = make_av(_SOURCE)


@register
class ClassifyCategoryCommand(CommandDef):
    name = "CLASSIFY::category"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFY::category",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows to set or add a category name to the classification.",
                synopsis=("CLASSIFY::category ('set' | 'add') CLASSIFY_CATEGORY_NAME",),
                snippet=(
                    "This command allows you to set or add a category name to the\n"
                    "classification.\n"
                    "\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work.\n"
                    "\n"
                    "CLASSIFY::category set <category_name>\n"
                    "\n"
                    "     * will immediately classify flow as category_name. The classification\n"
                    "       by the classification engine will be bypassed. Flow will have the unknown application classification token.\n"
                    "\n"
                    "CLASSIFY::category add <category_name>\n"
                    "\n"
                    "     * will add a category classification token to the final\n"
                    "       classification result issued by the classification engine."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFY::category ('set' | 'add') CLASSIFY_CATEGORY_NAME",
                    arg_values={
                        0: (
                            _av(
                                "set",
                                "CLASSIFY::category set",
                                "CLASSIFY::category ('set' | 'add') CLASSIFY_CATEGORY_NAME",
                            ),
                            _av(
                                "add",
                                "CLASSIFY::category add",
                                "CLASSIFY::category ('set' | 'add') CLASSIFY_CATEGORY_NAME",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP"}), also_in=frozenset({"CLIENT_ACCEPTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
