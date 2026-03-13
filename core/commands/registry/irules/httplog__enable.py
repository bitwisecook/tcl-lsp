# Generated from F5 iRules reference documentation -- do not edit manually.
"""HTTPLOG::enable -- F5 iRules command `HTTPLOG::enable`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTPLOG__enable.html"


@register
class HttplogEnableCommand(CommandDef):
    name = "HTTPLOG::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTPLOG::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `HTTPLOG::enable`.",
                synopsis=("HTTPLOG::enable",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTPLOG::enable",
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
