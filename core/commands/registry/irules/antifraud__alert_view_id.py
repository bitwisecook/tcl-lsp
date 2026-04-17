# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_view_id -- Returns or sets the configured URL and view which triggered this alert."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_view_id.html"


@register
class AntifraudAlertViewIdCommand(CommandDef):
    name = "ANTIFRAUD::alert_view_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_view_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets the configured URL and view which triggered this alert.",
                synopsis=("ANTIFRAUD::alert_view_id (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_view_id ;\n"
                    "                Returns the configured URL and view which triggered this alert. Empty if not a view.\n"
                    "\n"
                    "            ANTIFRAUD::alert_view_id VALUE ;\n"
                    "                Sets the configured URL and view which triggered this alert. Empty if not a view."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert View ID: [ANTIFRAUD::alert_view_id]."\n'
                    "                ANTIFRAUD::alert_view_id new_value\n"
                    '                log local0. "new Alert View ID: [ANTIFRAUD::alert_view_id]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_view_id ; Returns the configured URL and view which triggered this alert. Empty if not a view.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_view_id (VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ANTIFRAUD"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
