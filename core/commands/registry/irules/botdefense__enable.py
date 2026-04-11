# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::enable -- Enables processing by Bot Defense on the connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__enable.html"


@register
class BotdefenseEnableCommand(CommandDef):
    name = "BOTDEFENSE::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables processing by Bot Defense on the connection.",
                synopsis=("BOTDEFENSE::enable",),
                snippet="Enables processing and blocking of the request by Bot Defense, for the duration of the current TCP connection, or until BOTDEFENSE::disable is called.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Re-enable Bot Defense on the connection if a request arrives with a certain URL prefix.\n"
                    "when HTTP_REQUEST {\n"
                    '    if {[HTTP::uri] starts_with "/t/"} {\n'
                    "        BOTDEFENSE::enable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::enable",
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
