# Generated from F5 iRules reference documentation -- do not edit manually.
"""RADIUS::subscriber -- RADIUS::subscriber"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RADIUS__subscriber.html"


@register
class RadiusSubscriberCommand(CommandDef):
    name = "RADIUS::subscriber"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RADIUS::subscriber",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="RADIUS::subscriber",
                synopsis=("RADIUS::subscriber (SUBSCRIBER_ID)?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RADIUS::subscriber (SUBSCRIBER_ID)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset(
                    {
                        "CLIENT_ACCEPTED",
                        "CLIENT_CLOSED",
                        "CLIENT_DATA",
                        "SERVER_CLOSED",
                        "SERVER_CONNECTED",
                        "SERVER_DATA",
                    }
                )
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
