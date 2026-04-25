# Generated from F5 iRules reference documentation -- do not edit manually.
"""DEMANGLE::enable -- F5 iRules command `DEMANGLE::enable`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DEMANGLE__enable.html"


@register
class DemangleEnableCommand(CommandDef):
    name = "DEMANGLE::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DEMANGLE::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `DEMANGLE::enable`.",
                synopsis=("DEMANGLE::enable",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DEMANGLE::enable",
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
