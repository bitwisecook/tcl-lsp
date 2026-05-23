# Generated from F5 iRules reference documentation -- do not edit manually.
"""AVR::log -- Log an event for stats."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AVR__log.html"


@register
class AvrLogCommand(CommandDef):
    name = "AVR::log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AVR::log",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Log an event for stats.",
                synopsis=("AVR::log",),
                snippet="AVR::log ssl_orchestrator Log ssl_orchestrator stats to AVR",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AVR::log",
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
