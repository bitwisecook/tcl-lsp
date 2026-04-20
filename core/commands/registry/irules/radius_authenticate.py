# Generated from F5 iRules reference documentation -- do not edit manually.
"""radius_authenticate -- radius_authenticate command creates a RADIUS access request message, sends to the given RADIUS server and returns the request result."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/radius_authenticate.html"


@register
class RadiusAuthenticateCommand(CommandDef):
    name = "radius_authenticate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="radius_authenticate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="radius_authenticate command creates a RADIUS access request message, sends to the given RADIUS server and returns the request result.",
                synopsis=("radius_authenticate",),
                snippet="radius_authenticate command creates a RADIUS access request message, sends to the given RADIUS server and returns the request result.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="radius_authenticate",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
