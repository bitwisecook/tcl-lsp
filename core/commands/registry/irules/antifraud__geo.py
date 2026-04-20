# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::geo -- Returns L3 geoIP and geolocation collected by client."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__geo.html"


@register
class AntifraudGeoCommand(CommandDef):
    name = "ANTIFRAUD::geo"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::geo",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns L3 geoIP and geolocation collected by client.",
                synopsis=("ANTIFRAUD::geo",),
                snippet="Returns L3 geoIP and geolocation collected by client.",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "geolocation: [ANTIFRAUD::geo]."\n'
                    "            }"
                ),
                return_value="Returns L3 geoIP and geolocation collected by client.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::geo",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
