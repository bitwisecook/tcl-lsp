# Enriched from F5 iRules reference documentation.
"""AVR::disable -- Disables the AVR plugin for the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AVR__disable.html"


@register
class AvrDisableCommand(CommandDef):
    name = "AVR::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AVR::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables the AVR plugin for the current connection.",
                synopsis=("AVR::disable",),
                snippet=(
                    "Disables the AVR plugin for the current connection. AVR will remain\n"
                    "disabled on the current connection until it is closed or\n"
                    "AVR::enable is called. This means that the connection will not be\n"
                    "counted by AVR and thus excluded from statistics gathering."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AVR::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
