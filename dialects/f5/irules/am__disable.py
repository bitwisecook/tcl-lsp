# Generated from F5 iRules reference documentation -- do not edit manually.
"""AM::disable -- F5 iRules command `AM::disable`."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AM__disable.html"


@register
class AmDisableCommand(CommandDef):
    name = "AM::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AM::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `AM::disable`.",
                synopsis=("AM::disable",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AM::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
