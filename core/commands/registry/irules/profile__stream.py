# Enriched from F5 iRules reference documentation.
"""PROFILE::stream -- Returns the value of a Stream profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__stream.html"


@register
class ProfileStreamCommand(CommandDef):
    name = "PROFILE::stream"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::stream",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of a Stream profile setting.",
                synopsis=("PROFILE::stream ATTR",),
                snippet="Returns the current value of the specified setting in the assigned Stream profile.",
                source=_SOURCE,
                return_value="Returns the current value of the specified setting in the assigned Stream profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::stream ATTR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
