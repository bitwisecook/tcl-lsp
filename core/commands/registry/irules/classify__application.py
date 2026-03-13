# Enriched from F5 iRules reference documentation.
"""CLASSIFY::application -- Allows to set or add an app name to the classification."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFY__application.html"


_av = make_av(_SOURCE)


@register
class ClassifyApplicationCommand(CommandDef):
    name = "CLASSIFY::application"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFY::application",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows to set or add an app name to the classification.",
                synopsis=("CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME",),
                snippet=(
                    "This command allows you to set or add an app name to the\n"
                    "classification.\n"
                    "\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work.\n"
                    "\n"
                    "CLASSIFY::application set <app_name>\n"
                    "\n"
                    "     * will immediately classify flow as app_name. The classification by\n"
                    "       the classification engine will be bypassed. The category classification token this app_name belongs to will also be attached.\n"
                    "\n"
                    "CLASSIFY::application add <app_name>\n"
                    "\n"
                    "     * adds an application classification token to the final\n"
                    "       classification result issued by the classification engine."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME",
                    arg_values={
                        0: (
                            _av(
                                "set",
                                "CLASSIFY::application set",
                                "CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME",
                            ),
                            _av(
                                "add",
                                "CLASSIFY::application add",
                                "CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME",
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
