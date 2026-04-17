# Generated from F5 iRules reference documentation -- do not edit manually.
"""AM::age -- F5 iRules command `AM::age`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AM__age.html"


@register
class AmAgeCommand(CommandDef):
    name = "AM::age"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AM::age",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `AM::age`.",
                synopsis=("AM::age",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AM::age",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
