# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_transaction_id -- Returns or sets alert HTTP transaction ID."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_transaction_id.html"


@register
class AntifraudAlertTransactionIdCommand(CommandDef):
    name = "ANTIFRAUD::alert_transaction_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_transaction_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets alert HTTP transaction ID.",
                synopsis=("ANTIFRAUD::alert_transaction_id (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_transaction_id ;\n"
                    "                Returns alert HTTP transaction ID.\n"
                    "\n"
                    "            ANTIFRAUD::alert_transaction_id VALUE ;\n"
                    "                Sets alert HTTP transaction ID."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert transaction ID: [ANTIFRAUD::alert_transaction_id]."\n'
                    "                ANTIFRAUD::alert_transaction_id new_value\n"
                    '                log local0. "new Alert transaction ID: [ANTIFRAUD::alert_transaction_id]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_transaction_id ; Returns alert HTTP transaction ID.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_transaction_id (VALUE)?",
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
