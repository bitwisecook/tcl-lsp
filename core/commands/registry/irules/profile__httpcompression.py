# Enriched from F5 iRules reference documentation.
"""PROFILE::httpcompression -- Returns the value of an HTTP compression profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__httpcompression.html"


@register
class ProfileHttpcompressionCommand(CommandDef):
    name = "PROFILE::httpcompression"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::httpcompression",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of an HTTP compression profile setting.",
                synopsis=("PROFILE::httpcompression ATTR",),
                snippet="Returns the current value of the specified setting in an assigned HTTP compression profile.",
                source=_SOURCE,
                return_value="Returns the current value of the specified setting in an assigned HTTP compression profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::httpcompression ATTR",
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
