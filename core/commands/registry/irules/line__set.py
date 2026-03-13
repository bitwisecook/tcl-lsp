# Generated from F5 iRules reference documentation -- do not edit manually.
"""LINE::set -- F5 iRules command `LINE::set`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LINE__set.html"


@register
class LineSetCommand(CommandDef):
    name = "LINE::set"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LINE::set",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `LINE::set`.",
                synopsis=("LINE::set",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LINE::set",
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
