# Enriched from F5 iRules reference documentation.
"""ASM::disable -- Disables plugin processing on the connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__disable.html"


@register
class AsmDisableCommand(CommandDef):
    name = "ASM::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables plugin processing on the connection.",
                synopsis=("ASM::disable",),
                snippet=(
                    "Disables the ASM plugin processing for the current TCP connection.\n"
                    "ASM will remain disabled on the current TCP connection until it is closed or\n"
                    "ASM::enable is called."
                ),
                source=_SOURCE,
                examples=(
                    "# for 11.4.0+ the command should be used in HTTP_REQUEST event\n"
                    "when HTTP_CLASS_SELECTED {\n"
                    "  ASM::enable\n"
                    "  # Disable ASM for HTTP paths ending in .jpg\n"
                    '  if { [HTTP::path] ends_with ".jpg" } {\n'
                    "    ASM::disable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            xc_translatable=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
