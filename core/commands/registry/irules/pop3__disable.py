# Enriched from F5 iRules reference documentation.
"""POP3::disable -- Disable POP3 (STARTTLS for POP3)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/POP3__disable.html"


@register
class Pop3DisableCommand(CommandDef):
    name = "POP3::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="POP3::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disable POP3 (STARTTLS for POP3).",
                synopsis=("POP3::disable",),
                snippet="Disable POP3 (STARTTLS for POP3)",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { !([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    POP3::disable\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="POP3::disable",
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
