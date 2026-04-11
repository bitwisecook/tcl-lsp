# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::enable_log -- Enables Anti-Fraud TMM logs for the current transaction."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__enable_log.html"


@register
class AntifraudEnableLogCommand(CommandDef):
    name = "ANTIFRAUD::enable_log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::enable_log",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables Anti-Fraud TMM logs for the current transaction.",
                synopsis=("ANTIFRAUD::enable_log (LOG_LEVEL)?",),
                snippet=(
                    "ANTIFRAUD::enable_log\n"
                    "                Enables Anti-Fraud TMM logs at 'Informational' (default) log level for the current transaction.\n"
                    "\n"
                    "            ANTIFRAUD::enable_log LOG_LEVEL ;\n"
                    "                Enables Anti-Fraud TMM logs at 'LOG_LEVEL' (can be any of: 'Error'/'Warning'/'Notice'/'Informational'/'Debug') log level for the current transaction."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '                if { [HTTP::header exists "Antifraud-Enable-log" ] } {\n'
                    "                    ANTIFRAUD::enable_log\n"
                    '                    log local0. "Logs enabled"\n'
                    "                }\n"
                    "            }"
                ),
                return_value="ANTIFRAUD::enable_log No return value (enables Anti-Fraud TMM logs at default log level for the current transaction).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::enable_log (LOG_LEVEL)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
