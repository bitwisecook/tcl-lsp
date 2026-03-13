# Enriched from F5 iRules reference documentation.
"""NTLM::disable -- Disables processing for NTLM."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NTLM__disable.html"


@register
class NtlmDisableCommand(CommandDef):
    name = "NTLM::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NTLM::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables processing for NTLM.",
                synopsis=("NTLM::disable",),
                snippet="Disables processing for NTLM",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    '    if { [string tolower [HTTP::header values "WWW-Authenticate"]] contains "negotiate"} {\n'
                    "        ONECONNECT::detach disable\n"
                    "        NTLM::disable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NTLM::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
