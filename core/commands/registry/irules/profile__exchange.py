# Generated from F5 iRules reference documentation -- do not edit manually.
"""PROFILE::exchange -- F5 iRules command `PROFILE::exchange`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__exchange.html"


@register
class ProfileExchangeCommand(CommandDef):
    name = "PROFILE::exchange"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::exchange",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `PROFILE::exchange`.",
                synopsis=("PROFILE::exchange ATTR",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::exchange ATTR",
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
