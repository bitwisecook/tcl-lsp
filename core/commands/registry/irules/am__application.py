# Generated from F5 iRules reference documentation -- do not edit manually.
"""AM::application -- F5 iRules command `AM::application`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AM__application.html"


@register
class AmApplicationCommand(CommandDef):
    name = "AM::application"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AM::application",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `AM::application`.",
                synopsis=("AM::application",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AM::application",
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
