# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_transaction_data -- Returns or sets key-value list of all parameters marked to be attached."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_transaction_data.html"


@register
class AntifraudAlertTransactionDataCommand(CommandDef):
    name = "ANTIFRAUD::alert_transaction_data"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_transaction_data",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets key-value list of all parameters marked to be attached.",
                synopsis=("ANTIFRAUD::alert_transaction_data (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_transaction_data ;\n"
                    "                Returns key-value list of all parameters marked to be attached.\n"
                    "\n"
                    "            ANTIFRAUD::alert_transaction_data VALUE ;\n"
                    "                Sets key-value list of all parameters marked to be attached."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert transaction data: [ANTIFRAUD::alert_transaction_data]."\n'
                    "                ANTIFRAUD::alert_transaction_data new_value\n"
                    '                log local0. "new Alert transaction data: [ANTIFRAUD::alert_transaction_data]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_transaction_data ; Returns key-value list of all parameters marked to be attached.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_transaction_data (VALUE)?",
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
