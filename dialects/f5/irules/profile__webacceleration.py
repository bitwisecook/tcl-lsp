# Enriched from F5 iRules reference documentation.
"""PROFILE::webacceleration -- Returns the value of an web acceleration profile setting."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__webacceleration.html"


@register
class ProfileWebaccelerationCommand(CommandDef):
    name = "PROFILE::webacceleration"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::webacceleration",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of an web acceleration profile setting.",
                synopsis=("PROFILE::webacceleration ATTR",),
                snippet="Returns the value of an web acceleration profile setting",
                source=_SOURCE,
                return_value="Returns the value of an web acceleration profile setting",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::webacceleration ATTR",
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
