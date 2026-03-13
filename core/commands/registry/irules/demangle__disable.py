# Generated from F5 iRules reference documentation -- do not edit manually.
"""DEMANGLE::disable -- F5 iRules command `DEMANGLE::disable`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DEMANGLE__disable.html"


@register
class DemangleDisableCommand(CommandDef):
    name = "DEMANGLE::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DEMANGLE::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `DEMANGLE::disable`.",
                synopsis=("DEMANGLE::disable",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DEMANGLE::disable",
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
