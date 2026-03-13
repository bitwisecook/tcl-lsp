# Enriched from F5 iRules reference documentation.
"""PROFILE::oneconnect -- Returns the value of a Oneconnect profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__oneconnect.html"


@register
class ProfileOneconnectCommand(CommandDef):
    name = "PROFILE::oneconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::oneconnect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of a Oneconnect profile setting.",
                synopsis=("PROFILE::oneconnect ATTR",),
                snippet="Returns the current value of the specified setting in the assigned Oneconnect profile.",
                source=_SOURCE,
                return_value="Returns the current value of the specified setting in the assigned Oneconnect profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::oneconnect ATTR",
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
