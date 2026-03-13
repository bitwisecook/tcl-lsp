# Enriched from F5 iRules reference documentation.
"""SMTPS::enable -- Enable SMTPS (STARTTLS for SMTP)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SMTPS__enable.html"


@register
class SmtpsEnableCommand(CommandDef):
    name = "SMTPS::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SMTPS::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable SMTPS (STARTTLS for SMTP).",
                synopsis=("SMTPS::enable",),
                snippet="Enable SMTPS (STARTTLS for SMTP)",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    SMTPS::enable\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SMTPS::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
