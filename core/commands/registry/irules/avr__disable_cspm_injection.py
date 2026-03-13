# Enriched from F5 iRules reference documentation.
"""AVR::disable_cspm_injection -- Disables CSPM injection for the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AVR__disable_cspm_injection.html"


@register
class AvrDisableCspmInjectionCommand(CommandDef):
    name = "AVR::disable_cspm_injection"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AVR::disable_cspm_injection",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables CSPM injection for the current connection.",
                synopsis=("AVR::disable_cspm_injection",),
                snippet="The CSPM (Client Side Performance Monitoring) feature injects JavaScript into HTTP responses to track the Page Load Time metric. This command disables CSPM JavaScropt injection.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    if { [HTTP::status] == 404 } {\n"
                    "        AVR::disable_cspm_injection\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AVR::disable_cspm_injection",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
