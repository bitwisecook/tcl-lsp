# Enriched from F5 iRules reference documentation.
"""AVR::enable -- Enables the AVR plugin for the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AVR__enable.html"


@register
class AvrEnableCommand(CommandDef):
    name = "AVR::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AVR::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables the AVR plugin for the current connection.",
                synopsis=("AVR::enable",),
                snippet=(
                    "Enables the AVR plugin for the current connection. AVR will remain\n"
                    "enabled on the current connection until it is closed or\n"
                    "AVR::disable is called.\n"
                    "\n"
                    "Note that enabling AVR alone within the iRule only ensures the\n"
                    "message reaches the AVR plugin, it doesn't ensure that statistics\n"
                    "will be gathered."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AVR::enable",
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
